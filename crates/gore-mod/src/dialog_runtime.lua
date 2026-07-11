-- Generated configuration is inserted below by gore-mod. Keep this runtime self-contained: the
-- bundle must not require a separately installed shared Lua library.
do
local PREFIX = "[GoreDialogRuntime]"
local VERSION = "1"
local MOD_NAME = __GORE_DIALOG_MOD_NAME__
local REGISTRATIONS = {
__GORE_DIALOG_REGISTRATIONS__}

local SHOW_CONVERSATION_PATH =
    "/Script/G1R.GameplayAbilityConversationV2WithUI:ClientShowConversationUI"
local SHOW_CHOICE_PATH =
    "/Script/G1R.GameplayAbilityConversationV2WithUI:ClientShowChoiceUI"
local RENDER_TOPICS_PATH =
    "/Script/G1R.ConversationWidget:OnShowTopicSelection"

local MAX_PARTICIPANTS = 16
local MAX_VISIBLE_TOPICS = 64

local next_attempt_id = 0
-- Sessions retain only integer addresses, indices, counts, and strings. Never retain UObjects.
local active_session = nil
local pending_render_session = nil

local function log_global(status, detail)
    print(string.format(
        "%s mod=%q status=%s %s\n",
        PREFIX,
        MOD_NAME,
        tostring(status),
        tostring(detail or "")
    ))
end

local function log(registration_id, attempt_id, status, detail)
    print(string.format(
        "%s mod=%q registration=%q attempt=%d status=%s %s\n",
        PREFIX,
        MOD_NAME,
        tostring(registration_id),
        attempt_id,
        tostring(status),
        tostring(detail or "")
    ))
end

local function unwrap(value)
    if value == nil then return nil end
    local ok, actual = pcall(function() return value:get() end)
    if ok and actual ~= nil then return actual end
    return value
end

local function is_valid(value)
    value = unwrap(value)
    if value == nil then return false end
    local ok, result = pcall(function() return value:IsValid() end)
    return ok and result == true
end

local function address_of(value)
    value = unwrap(value)
    if not is_valid(value) then return nil end
    local ok, address = pcall(function() return value:GetAddress() end)
    -- UE4SS exposes user-space pointers as exactly representable Lua numbers on this runtime.
    -- Reject fractional, negative, NaN/infinite, and precision-losing values before retaining one
    -- across callbacks as an identity token.
    if not ok or type(address) ~= "number" or address <= 0 or
        address ~= math.floor(address) or address > 9007199254740991 then
        return nil
    end
    return address
end

local function find_class(path)
    local ok, found = pcall(function() return StaticFindObject(path) end)
    found = ok and unwrap(found) or nil
    if not is_valid(found) then return nil end

    local is_class = false
    pcall(function() is_class = found:IsAnyClass() end)
    if not is_class then pcall(function() is_class = found:IsClass() end) end
    if not is_class then return nil end
    return found
end

local function lower_name(value)
    value = unwrap(value)
    if value == nil then return "" end
    local ok, result = pcall(function() return value:ToString() end)
    if not ok or type(result) ~= "string" then return "" end
    return string.lower(result)
end

local function checked_array(value, maximum)
    local array = unwrap(value)
    if array == nil then return nil, nil, "nil-array" end
    local ok, count = pcall(function() return array:GetArrayNum() end)
    if not ok or type(count) ~= "number" or count < 0 or count ~= math.floor(count) then
        return nil, nil, "invalid-count"
    end
    if count > maximum then
        return nil, nil, "count-over-limit:" .. tostring(count)
    end
    return array, count, nil
end

local function has_exact_participant(group, expected_name)
    local participants = nil
    local property_ok = pcall(function() participants = group.Participants end)
    if not property_ok then return false, "participants-unreadable" end

    local array, expected_count, array_error =
        checked_array(participants, MAX_PARTICIPANTS)
    if array == nil then return false, array_error end
    if expected_count < 2 then
        return false, "participant-count:" .. tostring(expected_count)
    end

    local invalid = 0
    local target_count = 0
    for index = 1, expected_count do
        -- GetArrayNum established the logical range. Never read Num + 1 and do not use the
        -- crash-prone UE4SS 3.0.1 TArray iterator.
        local read_ok, wrapped_participant = pcall(function() return array[index] end)
        local participant = read_ok and unwrap(wrapped_participant) or nil
        if not is_valid(participant) then
            invalid = invalid + 1
        else
            local participant_name = nil
            local name_ok = pcall(function()
                participant_name = group:GetParticipantName(participant)
            end)
            if not name_ok then
                return false, "participant-name-unreadable:" .. tostring(index)
            end
            local normalized_name = lower_name(participant_name)
            if normalized_name == "" then
                return false, "participant-name-unreadable:" .. tostring(index)
            end
            if normalized_name == expected_name then
                target_count = target_count + 1
            end
        end
    end
    if invalid ~= 0 then return false, "invalid-participants:" .. tostring(invalid) end
    if target_count ~= 1 then return false, "target-participants:" .. tostring(target_count) end
    return true, nil
end

local function find_topic_instance(topic_set, topic_class)
    local expected_class_address = address_of(topic_class)
    if expected_class_address == nil then return nil, "class-unaddressable" end
    local ok, raw_topic = pcall(function()
        return topic_set:FindTopicInstanceOfClass(topic_class)
    end)
    if not ok then return nil, "error" end
    if raw_topic == nil then return nil, "missing" end
    local topic = unwrap(raw_topic)
    if not is_valid(topic) then return nil, "invalid-result" end
    local actual_class = nil
    local class_ok = pcall(function() actual_class = topic:GetClass() end)
    if not class_ok then return nil, "class-unreadable" end
    local actual_class_address = address_of(actual_class)
    if actual_class_address == nil or actual_class_address ~= expected_class_address then
        return nil, "class-mismatch"
    end
    return topic, "found"
end

local function resolve_context(ability, registration)
    ability = unwrap(ability)
    local ability_address = address_of(ability)
    if ability_address == nil then return nil, "invalid-ability" end

    local group = nil
    local group_ok = pcall(function() group = ability.ConversationGroup end)
    group = group_ok and unwrap(group) or nil
    local group_address = address_of(group)
    if group_address == nil then return nil, "invalid-conversation-group" end

    local end_requested = nil
    local end_ok = pcall(function() end_requested = group.bEndRequested end)
    if not end_ok or type(end_requested) ~= "boolean" then
        return nil, "end-request-state-unreadable"
    end
    if end_requested then return nil, "end-already-requested" end

    local participant_ok, participant_error =
        has_exact_participant(group, registration.participant_name)
    if not participant_ok then return nil, participant_error end

    local topic_set = nil
    local set_ok = pcall(function() topic_set = group.TopicSet end)
    topic_set = set_ok and unwrap(topic_set) or nil
    local topic_set_address = address_of(topic_set)
    if topic_set_address == nil then return nil, "invalid-topic-set" end

    -- The exact vanilla sentinel in this exact set is an independent locality proof.
    local sentinel_class = find_class(registration.sentinel_class_path)
    if sentinel_class == nil then return nil, "sentinel-class-missing" end
    local _, sentinel_state = find_topic_instance(topic_set, sentinel_class)
    if sentinel_state ~= "found" then
        return nil, "sentinel-topic-" .. tostring(sentinel_state)
    end

    return {
        ability_address = ability_address,
        group_address = group_address,
        topic_set = topic_set,
        topic_set_address = topic_set_address,
    }, nil
end

local function resolve_widget_address(ability)
    ability = unwrap(ability)
    if not is_valid(ability) then return nil end
    local widget = nil
    local widget_ok = pcall(function() widget = ability.PlayerConversationWidget end)
    widget = widget_ok and unwrap(widget) or nil
    return address_of(widget)
end

local function add_or_reuse_topic(context, topic_class)
    local existing, existing_state = find_topic_instance(context.topic_set, topic_class)
    if existing_state == "found" then return existing, "reused" end
    if existing_state ~= "missing" then
        return nil, "topic-lookup-" .. tostring(existing_state)
    end

    -- Sole gameplay mutation. It runs only after participant and sentinel locality proofs pass.
    local replacement_name = FName("None", EFindName.FNAME_Find)
    if lower_name(replacement_name) ~= "none" then
        return nil, "none-fname-roundtrip-failed"
    end
    local ok, added_or_error = pcall(function()
        return context.topic_set:AddTopic(topic_class, replacement_name)
    end)
    local added = ok and unwrap(added_or_error) or nil
    if not is_valid(added) then return nil, "add-failed:" .. tostring(added_or_error) end
    return added, "added"
end

local function on_show_conversation(context_param)
    active_session = nil
    pending_render_session = nil
    next_attempt_id = next_attempt_id + 1
    local attempt_id = next_attempt_id
    local ability = unwrap(context_param)
    local ability_address = address_of(ability)
    if ability_address == nil then
        log_global("SKIP", "attempt=" .. attempt_id .. " reason=invalid-ability")
        return
    end

    -- Resolve every locality proof and authored class before the first mutation. This prevents an
    -- earlier registration's newly-added topic from satisfying a later registration's sentinel.
    -- Registrations are intentionally independent: a class can load lazily and one NPC/state may
    -- be absent, so a failed entry is skipped for this attempt without disabling unrelated entries.
    local candidates = {}
    for registration_index, registration in ipairs(REGISTRATIONS) do
        local context, context_error = resolve_context(ability, registration)
        if context == nil then
            log(registration.id, attempt_id, "SKIP", "reason=" .. tostring(context_error))
        else
            local topic_class = find_class(registration.topic_class_path)
            local topic_class_address = address_of(topic_class)
            if topic_class_address == nil then
                log(registration.id, attempt_id, "SKIP", "reason=topic-class-missing")
            else
                candidates[#candidates + 1] = {
                    registration_index = registration_index,
                    context = context,
                    topic_class = topic_class,
                    topic_class_address = topic_class_address,
                }
            end
        end
    end

    local entries = {}
    for _, candidate in ipairs(candidates) do
        local registration = REGISTRATIONS[candidate.registration_index]
        local topic, add_result =
            add_or_reuse_topic(candidate.context, candidate.topic_class)
        local topic_address = address_of(topic)
        if topic_address == nil then
            log(registration.id, attempt_id, "FAIL", "reason=" .. tostring(add_result))
        else
            entries[#entries + 1] = {
                registration_index = candidate.registration_index,
                group_address = candidate.context.group_address,
                topic_set_address = candidate.context.topic_set_address,
                topic_class_address = candidate.topic_class_address,
                topic_address = topic_address,
                add_result = add_result,
            }
            log(registration.id, attempt_id, "ARMED", "mutation=" .. add_result)
        end
    end
    if #entries ~= 0 then
        active_session = {
            attempt_id = attempt_id,
            ability_address = ability_address,
            entries = entries,
        }
    end
end

local function inspect_visible_topic(expected, available_topics_param)
    local array, expected_count, array_error =
        checked_array(available_topics_param, MAX_VISIBLE_TOPICS)
    if array == nil then return false, array_error end

    local invalid = 0
    local identity_count = 0
    local class_count = 0
    local exact_count = 0
    for index = 1, expected_count do
        local read_ok, wrapped_topic = pcall(function() return array[index] end)
        local topic = read_ok and unwrap(wrapped_topic) or nil
        local topic_address = address_of(topic)
        if topic_address == nil then
            invalid = invalid + 1
        else
            local identity_match = topic_address == expected.topic_address
            if identity_match then identity_count = identity_count + 1 end
            local topic_class = nil
            pcall(function() topic_class = topic:GetClass() end)
            local class_match = address_of(topic_class) == expected.topic_class_address
            if class_match then class_count = class_count + 1 end
            if identity_match and class_match then exact_count = exact_count + 1 end
        end
    end
    if invalid ~= 0 then
        return false, "invalid-topics:" .. tostring(invalid), expected_count
    end
    local passed = identity_count == 1 and class_count == 1 and exact_count == 1
    return passed, string.format(
        "topics=%d identity_count=%d class_count=%d exact_count=%d",
        expected_count,
        identity_count,
        class_count,
        exact_count
    ), expected_count
end

local function on_show_choice(context_param, available_topics_param)
    local expected = active_session
    active_session = nil
    pending_render_session = nil
    if expected == nil then return end

    local ability = unwrap(context_param)
    if address_of(ability) ~= expected.ability_address then
        log_global("FAIL", "attempt=" .. expected.attempt_id .. " reason=ability-changed")
        return
    end
    local widget_address = resolve_widget_address(ability)
    if widget_address == nil then
        log_global("FAIL", "attempt=" .. expected.attempt_id .. " stage=choice reason=widget-unavailable")
        return
    end

    local topic_count = nil
    for _, entry in ipairs(expected.entries) do
        local registration = REGISTRATIONS[entry.registration_index]
        local context, context_error = resolve_context(ability, registration)
        if context == nil then
            log(registration.id, expected.attempt_id, "FAIL", "stage=choice reason=context-changed:" .. tostring(context_error))
            return
        end
        if context.group_address ~= entry.group_address or
            context.topic_set_address ~= entry.topic_set_address then
            log(registration.id, expected.attempt_id, "FAIL", "stage=choice reason=context-identity-changed")
            return
        end
        local passed, detail, count = inspect_visible_topic(entry, available_topics_param)
        if not passed then
            log(registration.id, expected.attempt_id, "FAIL", "stage=choice mutation=" .. entry.add_result .. " " .. tostring(detail))
            return
        end
        if topic_count ~= nil and topic_count ~= count then
            log(registration.id, expected.attempt_id, "FAIL", "stage=choice reason=count-inconsistent")
            return
        end
        topic_count = count
        log(registration.id, expected.attempt_id, "CHOICE_PASS", "mutation=" .. entry.add_result .. " " .. tostring(detail))
    end

    expected.widget_address = widget_address
    expected.choice_topic_count = topic_count
    pending_render_session = expected
end

local function on_render_topics(widget_context_param, topics_to_show_param)
    local expected = pending_render_session
    pending_render_session = nil
    if expected == nil then return end
    if address_of(unwrap(widget_context_param)) ~= expected.widget_address then
        log_global("FAIL", "attempt=" .. expected.attempt_id .. " stage=render reason=widget-changed")
        return
    end

    for _, entry in ipairs(expected.entries) do
        local registration = REGISTRATIONS[entry.registration_index]
        local passed, detail, topic_count = inspect_visible_topic(entry, topics_to_show_param)
        if not passed then
            log(registration.id, expected.attempt_id, "FAIL", "stage=render mutation=" .. entry.add_result .. " " .. tostring(detail))
            return
        end
        if topic_count ~= expected.choice_topic_count then
            log(registration.id, expected.attempt_id, "FAIL", "stage=render reason=count-changed:" .. tostring(topic_count) .. "/" .. tostring(expected.choice_topic_count))
            return
        end
        log(registration.id, expected.attempt_id, "RENDER_PASS", "mutation=" .. entry.add_result .. " " .. tostring(detail))
    end
end

local function protected_show_conversation(context_param)
    local ok, failure = pcall(on_show_conversation, context_param)
    if not ok then
        active_session = nil
        pending_render_session = nil
        log_global("ERROR", "hook=show-conversation reason=" .. tostring(failure))
    end
end

local function protected_show_choice(context_param, available_topics_param)
    local ok, failure = pcall(on_show_choice, context_param, available_topics_param)
    if not ok then
        active_session = nil
        pending_render_session = nil
        log_global("ERROR", "hook=show-choice reason=" .. tostring(failure))
    end
end

local function protected_render_topics(widget_context_param, topics_to_show_param)
    local ok, failure = pcall(on_render_topics, widget_context_param, topics_to_show_param)
    if not ok then
        pending_render_session = nil
        log_global("ERROR", "hook=render-topics reason=" .. tostring(failure))
    end
end

local function register_native_pre_hook(path, callback)
    local ok, pre_id, post_id = pcall(function() return RegisterHook(path, callback) end)
    if not ok or type(pre_id) ~= "number" then return nil, nil, tostring(pre_id) end
    return pre_id, post_id, nil
end

log_global("LOADED", "version=" .. VERSION .. " registrations=" .. tostring(#REGISTRATIONS))

-- Register both read-only proof stages first. The mutating hook is never installed unless both
-- observation hooks registered successfully.
local render_pre_id, render_post_id, render_error =
    register_native_pre_hook(RENDER_TOPICS_PATH, protected_render_topics)
if render_pre_id == nil then
    log_global("DISABLED", "reason=render-hook-failed:" .. tostring(render_error))
else
    local choice_pre_id, choice_post_id, choice_error =
        register_native_pre_hook(SHOW_CHOICE_PATH, protected_show_choice)
    if choice_pre_id == nil then
        log_global("DISABLED", "reason=choice-hook-failed:" .. tostring(choice_error))
    else
        local conversation_pre_id, conversation_post_id, conversation_error =
            register_native_pre_hook(SHOW_CONVERSATION_PATH, protected_show_conversation)
        if conversation_pre_id == nil then
            log_global("DISABLED", "reason=conversation-hook-failed:" .. tostring(conversation_error))
        else
            log_global("READY", string.format(
                "render_pre=%s render_post=%s choice_pre=%s choice_post=%s conversation_pre=%s conversation_post=%s",
                tostring(render_pre_id),
                tostring(render_post_id),
                tostring(choice_pre_id),
                tostring(choice_post_id),
                tostring(conversation_pre_id),
                tostring(conversation_post_id)
            ))
        end
    end
end
end
