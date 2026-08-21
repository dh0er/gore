#pragma once

// Narrow Unreal Core compatibility surface required by the pinned UNREANGEL
// AngelScript core. This is not an Unreal API implementation.

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <functional>
#include <limits>
#include <type_traits>
#include <utility>
#include <vector>

using int8 = std::int8_t;
using int16 = std::int16_t;
using int32 = std::int32_t;
using int64 = std::int64_t;
using uint8 = std::uint8_t;
using uint16 = std::uint16_t;
using uint32 = std::uint32_t;
using uint64 = std::uint64_t;
using SIZE_T = std::size_t;
using ANSICHAR = char;
using TCHAR = char;

inline constexpr int32 INDEX_NONE = -1;
inline constexpr int8 MAX_int8 = (std::numeric_limits<int8>::max)();
inline constexpr int8 MIN_int8 = (std::numeric_limits<int8>::min)();
inline constexpr int32 MAX_int32 = (std::numeric_limits<int32>::max)();

template <std::size_t InlineElements>
struct TInlineAllocator final {};

struct FDefaultAllocator final {};

template <typename KeyType, typename ValueType>
struct TPair {
    KeyType Key{};
    ValueType Value{};

    TPair() = default;
    TPair(const KeyType& key, const ValueType& value) : Key(key), Value(value) {}
    TPair(KeyType&& key, ValueType&& value)
        : Key(std::move(key)), Value(std::move(value)) {}

    friend bool operator==(const TPair& left, const TPair& right) {
        return left.Key == right.Key && left.Value == right.Value;
    }
    friend bool operator<(const TPair& left, const TPair& right) {
        return left.Key < right.Key || (!(right.Key < left.Key) && left.Value < right.Value);
    }
};

template <typename ValueType, typename Allocator = FDefaultAllocator>
class TArray {
public:
    using value_type = ValueType;
    using Storage = std::vector<ValueType>;
    using iterator = typename Storage::iterator;
    using const_iterator = typename Storage::const_iterator;
    using reference = typename Storage::reference;
    using const_reference = typename Storage::const_reference;

    TArray() = default;
    TArray(std::initializer_list<ValueType> values) : values_(values) {}

    [[nodiscard]] int32 Num() const noexcept {
        return static_cast<int32>(values_.size());
    }
    [[nodiscard]] bool IsEmpty() const noexcept { return values_.empty(); }
    [[nodiscard]] bool IsValidIndex(const int32 index) const noexcept {
        return index >= 0 && static_cast<std::size_t>(index) < values_.size();
    }

    void Reserve(const int32 count) {
        if (count > 0) {
            values_.reserve(static_cast<std::size_t>(count));
        }
    }
    void Reset(const int32 expected_count = 0) {
        values_.clear();
        Reserve(expected_count);
    }
    void Empty(const int32 expected_count = 0) {
        Storage empty;
        if (expected_count > 0) {
            empty.reserve(static_cast<std::size_t>(expected_count));
        }
        values_.swap(empty);
    }
    void SetNum(const int32 count) {
        values_.resize(static_cast<std::size_t>(count));
    }
    void SetNumZeroed(const int32 count) {
        values_.assign(static_cast<std::size_t>(count), ValueType{});
    }

    int32 Add(const ValueType& value) {
        values_.push_back(value);
        return Num() - 1;
    }
    int32 Add(ValueType&& value) {
        values_.push_back(std::move(value));
        return Num() - 1;
    }
    int32 AddUnique(const ValueType& value) {
        const auto found = Find(value);
        return found == INDEX_NONE ? Add(value) : found;
    }
    int32 AddUninitialized(const int32 count = 1) {
        const auto first_index = Num();
        values_.resize(values_.size() + static_cast<std::size_t>(count));
        return first_index;
    }
    template <typename... ArgumentTypes>
    int32 Emplace(ArgumentTypes&&... arguments) {
        values_.emplace_back(std::forward<ArgumentTypes>(arguments)...);
        return Num() - 1;
    }
    template <typename... ArgumentTypes>
    ValueType& Emplace_GetRef(ArgumentTypes&&... arguments) {
        values_.emplace_back(std::forward<ArgumentTypes>(arguments)...);
        return values_.back();
    }
    void Append(const TArray& other) {
        values_.insert(values_.end(), other.values_.begin(), other.values_.end());
    }

    [[nodiscard]] bool Contains(const ValueType& value) const {
        return Find(value) != INDEX_NONE;
    }
    [[nodiscard]] int32 Find(const ValueType& value) const {
        const auto found = std::find(values_.begin(), values_.end(), value);
        return found == values_.end()
            ? INDEX_NONE
            : static_cast<int32>(std::distance(values_.begin(), found));
    }
    [[nodiscard]] int32 IndexOfByKey(const ValueType& value) const { return Find(value); }

    int32 Remove(const ValueType& value) {
        const auto old_size = values_.size();
        values_.erase(std::remove(values_.begin(), values_.end(), value), values_.end());
        return static_cast<int32>(old_size - values_.size());
    }
    int32 RemoveSingle(const ValueType& value) {
        const auto found = std::find(values_.begin(), values_.end(), value);
        if (found == values_.end()) {
            return 0;
        }
        values_.erase(found);
        return 1;
    }
    int32 RemoveSingleSwap(const ValueType& value) {
        const auto found = Find(value);
        if (found == INDEX_NONE) {
            return 0;
        }
        RemoveAtSwap(found);
        return 1;
    }
    void RemoveAt(const int32 index, const int32 count = 1) {
        if (count <= 0) {
            return;
        }
        const auto first = values_.begin() + index;
        values_.erase(first, first + count);
    }
    void RemoveAtSwap(const int32 index, const int32 count = 1) {
        for (int32 removed = 0; removed < count; ++removed) {
            const auto current = static_cast<std::size_t>(index);
            if (current + 1U != values_.size()) {
                values_[current] = std::move(values_.back());
            }
            values_.pop_back();
        }
    }
    void Swap(const int32 left, const int32 right) {
        std::swap(values_[static_cast<std::size_t>(left)], values_[static_cast<std::size_t>(right)]);
    }

    ValueType Pop(const bool = true) {
        ValueType result = std::move(values_.back());
        values_.pop_back();
        return result;
    }
    reference Last(const int32 offset_from_end = 0) {
        return values_[values_.size() - 1U - static_cast<std::size_t>(offset_from_end)];
    }
    const_reference Last(const int32 offset_from_end = 0) const {
        return values_[values_.size() - 1U - static_cast<std::size_t>(offset_from_end)];
    }
    ValueType* GetData() noexcept { return values_.data(); }
    const ValueType* GetData() const noexcept { return values_.data(); }

    reference operator[](const int32 index) { return values_[static_cast<std::size_t>(index)]; }
    const_reference operator[](const int32 index) const {
        return values_[static_cast<std::size_t>(index)];
    }

    iterator begin() noexcept { return values_.begin(); }
    iterator end() noexcept { return values_.end(); }
    const_iterator begin() const noexcept { return values_.begin(); }
    const_iterator end() const noexcept { return values_.end(); }

private:
    Storage values_;
};

template <typename KeyType, typename ValueType>
class TMap {
public:
    using ElementType = TPair<KeyType, ValueType>;
    using Storage = std::vector<ElementType>;

    ValueType& Add(const KeyType& key, const ValueType& value) {
        if (auto* existing = Find(key)) {
            *existing = value;
            return *existing;
        }
        entries_.emplace_back(key, value);
        return entries_.back().Value;
    }
    ValueType& Add(const KeyType& key, ValueType&& value) {
        if (auto* existing = Find(key)) {
            *existing = std::move(value);
            return *existing;
        }
        entries_.emplace_back(key, std::move(value));
        return entries_.back().Value;
    }
    ValueType* Find(const KeyType& key) {
        const auto found = find_element(key);
        return found == entries_.end() ? nullptr : &found->Value;
    }
    const ValueType* Find(const KeyType& key) const {
        const auto found = find_element(key);
        return found == entries_.end() ? nullptr : &found->Value;
    }
    [[nodiscard]] ValueType FindRef(const KeyType& key) const {
        const auto* value = Find(key);
        return value == nullptr ? ValueType{} : *value;
    }
    ValueType& FindChecked(const KeyType& key) { return *Find(key); }
    const ValueType& FindChecked(const KeyType& key) const { return *Find(key); }
    [[nodiscard]] bool Contains(const KeyType& key) const { return Find(key) != nullptr; }
    int32 Remove(const KeyType& key) {
        const auto found = find_element(key);
        if (found == entries_.end()) {
            return 0;
        }
        entries_.erase(found);
        return 1;
    }
    void Reset() { entries_.clear(); }
    void Empty() { entries_.clear(); }
    void Reserve(const int32 count) {
        if (count > 0) {
            entries_.reserve(static_cast<std::size_t>(count));
        }
    }
    [[nodiscard]] int32 Num() const { return static_cast<int32>(entries_.size()); }

    auto begin() noexcept { return entries_.begin(); }
    auto end() noexcept { return entries_.end(); }
    auto begin() const noexcept { return entries_.begin(); }
    auto end() const noexcept { return entries_.end(); }

private:
    auto find_element(const KeyType& key) {
        return std::find_if(entries_.begin(), entries_.end(),
            [&key](const ElementType& element) { return element.Key == key; });
    }
    auto find_element(const KeyType& key) const {
        return std::find_if(entries_.begin(), entries_.end(),
            [&key](const ElementType& element) { return element.Key == key; });
    }

    Storage entries_;
};

template <typename KeyType, typename ValueType>
class TMultiMap {
public:
    using ElementType = TPair<KeyType, ValueType>;
    using Storage = std::vector<ElementType>;

    class ConstKeyIterator {
    public:
        ConstKeyIterator(const TMultiMap& owner, const KeyType& key)
            : owner_(&owner), key_(key) {
            seek();
        }
        explicit operator bool() const { return index_ < owner_->entries_.size(); }
        ConstKeyIterator& operator++() {
            ++index_;
            seek();
            return *this;
        }
        const ValueType& Value() const { return owner_->entries_[index_].Value; }

    private:
        void seek() {
            while (index_ < owner_->entries_.size() &&
                   !(owner_->entries_[index_].Key == key_)) {
                ++index_;
            }
        }

        const TMultiMap* owner_;
        KeyType key_;
        std::size_t index_ = 0U;
    };

    void Add(const KeyType& key, const ValueType& value) { entries_.emplace_back(key, value); }
    void AddUnique(const KeyType& key, const ValueType& value) {
        if (FindPair(key, value) == nullptr) {
            Add(key, value);
        }
    }
    int32 Remove(const KeyType& key, const ValueType& value) {
        const auto old_size = entries_.size();
        entries_.erase(std::remove_if(entries_.begin(), entries_.end(),
            [&key, &value](const ElementType& element) {
                return element.Key == key && element.Value == value;
            }), entries_.end());
        return static_cast<int32>(old_size - entries_.size());
    }
    ElementType* FindPair(const KeyType& key, const ValueType& value) {
        const auto found = std::find_if(entries_.begin(), entries_.end(),
            [&key, &value](const ElementType& element) {
                return element.Key == key && element.Value == value;
            });
        return found == entries_.end() ? nullptr : &*found;
    }
    const ElementType* FindPair(const KeyType& key, const ValueType& value) const {
        const auto found = std::find_if(entries_.begin(), entries_.end(),
            [&key, &value](const ElementType& element) {
                return element.Key == key && element.Value == value;
            });
        return found == entries_.end() ? nullptr : &*found;
    }
    ConstKeyIterator CreateConstKeyIterator(const KeyType& key) const {
        return ConstKeyIterator(*this, key);
    }
    void Empty() { entries_.clear(); }
    void Reserve(const int32 count) {
        if (count > 0) {
            entries_.reserve(static_cast<std::size_t>(count));
        }
    }

    auto begin() noexcept { return entries_.begin(); }
    auto end() noexcept { return entries_.end(); }
    auto begin() const noexcept { return entries_.begin(); }
    auto end() const noexcept { return entries_.end(); }

private:
    Storage entries_;
};

template <typename ValueType>
class TGuardValue final {
public:
    TGuardValue(ValueType& target, const ValueType& temporary)
        : target_(target), original_(target) {
        target_ = temporary;
    }
    ~TGuardValue() { target_ = original_; }
    TGuardValue(const TGuardValue&) = delete;
    TGuardValue& operator=(const TGuardValue&) = delete;

private:
    ValueType& target_;
    ValueType original_;
};

template <typename ValueType>
constexpr std::remove_reference_t<ValueType>&& MoveTemp(ValueType&& value) noexcept {
    return static_cast<std::remove_reference_t<ValueType>&&>(value);
}

template <typename ValueType>
constexpr ValueType Align(const ValueType value, const std::size_t alignment) {
    return static_cast<ValueType>((value + static_cast<ValueType>(alignment - 1U)) &
        ~static_cast<ValueType>(alignment - 1U));
}

inline uint32 HashCombineFast(const uint32 left, const uint32 right) noexcept {
    return left ^ (right + 0x9e3779b9U + (left << 6U) + (left >> 2U));
}

template <typename ValueType>
uint32 GetTypeHash(ValueType* value) noexcept {
    const auto bits = reinterpret_cast<std::uintptr_t>(value);
    return static_cast<uint32>(bits ^ (bits >> 32U));
}

template <typename ValueType>
uint32 GetTypeHash(const ValueType& value) noexcept {
    return static_cast<uint32>(std::hash<ValueType>{}(value));
}
