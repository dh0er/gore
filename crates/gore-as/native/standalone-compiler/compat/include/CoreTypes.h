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
#include <iterator>
#include <optional>
#include <type_traits>
#include <unordered_map>
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
    TPair(const KeyType& key, ValueType&& value)
        : Key(key), Value(std::move(value)) {}
    TPair(KeyType&& key, const ValueType& value)
        : Key(std::move(key)), Value(value) {}
    TPair(KeyType&& key, ValueType&& value)
        : Key(std::move(key)), Value(std::move(value)) {}

    friend bool operator==(const TPair& left, const TPair& right) {
        return left.Key == right.Key && left.Value == right.Value;
    }
    friend bool operator<(const TPair& left, const TPair& right) {
        return left.Key < right.Key || (!(right.Key < left.Key) && left.Value < right.Value);
    }
};

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

template <typename KeyType, typename ValueType>
uint32 GetTypeHash(const TPair<KeyType, ValueType>& value) noexcept {
    return HashCombineFast(GetTypeHash(value.Key), GetTypeHash(value.Value));
}

template <typename KeyType>
struct TCompatKeyHash {
    std::size_t operator()(const KeyType& key) const noexcept {
        return static_cast<std::size_t>(GetTypeHash(key));
    }
};

// UE's TMap/TMultiMap store elements in a TSparseArray. Iteration therefore follows stable sparse
// indices, and the next insertion reuses the most recently freed index. A plain std::vector erase
// changes every later index, while append-after-remove changes observable iteration order (for
// example scriptModulesByName Remove/Add during a module rename). This narrow storage reproduces
// the sparse-array lifetime/order contract needed by the pinned AngelScript core; hashing remains
// an acceleration detail because lookup equality is still checked on every candidate.
template <typename KeyType, typename ValueType>
class TSparsePairStorage {
public:
    using ElementType = TPair<KeyType, ValueType>;

private:
    struct Slot {
        std::optional<ElementType> element;
        std::uint64_t hash_chain_order = 0U;
    };

    struct IndexState {
        std::uint32_t active_count = 0U;
        std::uint32_t hash_bucket_count = 0U;
        std::uint64_t newest_hash_chain_order = 0U;
        std::unordered_map<KeyType, std::vector<std::size_t>, TCompatKeyHash<KeyType>>
            hash_index;
    };

public:
    TSparsePairStorage() = default;
    ~TSparsePairStorage() { delete index_state_; }
    TSparsePairStorage(const TSparsePairStorage& other)
        : slots_(other.slots_),
          free_indices_(other.free_indices_),
          index_state_(other.index_state_ == nullptr
              ? nullptr
              : new IndexState(*other.index_state_)) {}
    TSparsePairStorage& operator=(const TSparsePairStorage& other) {
        if (this != &other) {
            TSparsePairStorage replacement(other);
            *this = std::move(replacement);
        }
        return *this;
    }
    TSparsePairStorage(TSparsePairStorage&& other) noexcept
        : slots_(std::move(other.slots_)),
          free_indices_(std::move(other.free_indices_)),
          index_state_(std::exchange(other.index_state_, nullptr)) {}
    TSparsePairStorage& operator=(TSparsePairStorage&& other) noexcept {
        if (this != &other) {
            delete index_state_;
            slots_ = std::move(other.slots_);
            free_indices_ = std::move(other.free_indices_);
            index_state_ = std::exchange(other.index_state_, nullptr);
        }
        return *this;
    }

    template <bool IsConst>
    class Iterator {
    public:
        using Owner = std::conditional_t<IsConst, const TSparsePairStorage, TSparsePairStorage>;
        using iterator_category = std::forward_iterator_tag;
        using value_type = ElementType;
        using difference_type = std::ptrdiff_t;
        using reference = std::conditional_t<IsConst, const ElementType&, ElementType&>;
        using pointer = std::conditional_t<IsConst, const ElementType*, ElementType*>;

        Iterator() = default;
        Iterator(Owner* owner, const std::size_t index) : owner_(owner), index_(index) { seek(); }

        reference operator*() const { return *owner_->slots_[index_].element; }
        pointer operator->() const { return &*owner_->slots_[index_].element; }
        Iterator& operator++() {
            ++index_;
            seek();
            return *this;
        }
        Iterator operator++(int) {
            Iterator previous = *this;
            ++*this;
            return previous;
        }
        friend bool operator==(const Iterator& left, const Iterator& right) {
            return left.owner_ == right.owner_ && left.index_ == right.index_;
        }
        friend bool operator!=(const Iterator& left, const Iterator& right) {
            return !(left == right);
        }

    private:
        void seek() {
            if (owner_ == nullptr) return;
            while (index_ < owner_->slots_.size() &&
                   !owner_->slots_[index_].element.has_value()) {
                ++index_;
            }
        }

        Owner* owner_ = nullptr;
        std::size_t index_ = 0U;
    };

    using iterator = Iterator<false>;
    using const_iterator = Iterator<true>;

    template <typename... ArgumentTypes>
    std::size_t Emplace(ArgumentTypes&&... arguments) {
        std::size_t index = 0U;
        if (free_indices_.empty()) {
            index = slots_.size();
            slots_.emplace_back();
        } else {
            index = free_indices_.back();
            free_indices_.pop_back();
        }
        slots_[index].element.emplace(std::forward<ArgumentTypes>(arguments)...);
        set_active_count(active_count() + 1U);
        const std::uint32_t desired_hash_size =
            default_hash_bucket_count(active_count());
        if (desired_hash_size > hash_bucket_count()) {
            set_hash_bucket_count(desired_hash_size);
            rebuild_hash_chain_order();
            rebuild_hash_index();
        } else {
            if (index_state_->newest_hash_chain_order ==
                (std::numeric_limits<std::uint64_t>::max)()) {
                std::abort();
            }
            slots_[index].hash_chain_order = ++index_state_->newest_hash_chain_order;
            auto& indices = index_state_->hash_index[slots_[index].element->Key];
            indices.insert(indices.begin(), index);
        }
        return index;
    }

    void RemoveAt(const std::size_t index) {
        if (index >= slots_.size() || !slots_[index].element.has_value()) return;
        remove_from_hash_index(slots_[index].element->Key, index);
        slots_[index].element.reset();
        slots_[index].hash_chain_order = 0U;
        free_indices_.push_back(index);
        set_active_count(active_count() - 1U);
    }

    template <typename Predicate>
    std::size_t FindIndex(Predicate&& predicate) const {
        for (std::size_t index = 0U; index < slots_.size(); ++index) {
            if (slots_[index].element.has_value() &&
                predicate(*slots_[index].element)) return index;
        }
        return npos;
    }

    [[nodiscard]] std::size_t FindHashChainIndex(
        const KeyType& key,
        const std::uint64_t before_order =
            (std::numeric_limits<std::uint64_t>::max)()) const {
        if (active_count() == 0U) return npos;
        const auto found = index_state_->hash_index.find(key);
        if (found == index_state_->hash_index.end()) return npos;
        for (const std::size_t index : found->second) {
            if (slots_[index].hash_chain_order < before_order) return index;
        }
        return npos;
    }

    ElementType* At(const std::size_t index) {
        return index == npos || index >= slots_.size() ||
               !slots_[index].element.has_value()
            ? nullptr
            : &*slots_[index].element;
    }
    const ElementType* At(const std::size_t index) const {
        return index == npos || index >= slots_.size() ||
               !slots_[index].element.has_value()
            ? nullptr
            : &*slots_[index].element;
    }
    [[nodiscard]] bool IsAllocated(const std::size_t index) const {
        return index < slots_.size() && slots_[index].element.has_value();
    }
    [[nodiscard]] std::uint64_t HashChainOrderAt(
        const std::size_t index) const noexcept {
        return index < slots_.size() && slots_[index].element.has_value()
            ? slots_[index].hash_chain_order
            : 0U;
    }
    [[nodiscard]] std::size_t MaxIndex() const { return slots_.size(); }
    [[nodiscard]] int32 Num() const { return static_cast<int32>(active_count()); }
    void Reserve(const std::size_t count) {
        slots_.reserve(count);
        const std::uint32_t desired_hash_size = default_hash_bucket_count(count);
        if (desired_hash_size > hash_bucket_count()) {
            set_hash_bucket_count(desired_hash_size);
            rebuild_hash_chain_order();
            rebuild_hash_index();
        }
    }
    void Empty() {
        slots_.clear();
        free_indices_.clear();
        delete index_state_;
        index_state_ = nullptr;
    }

    iterator begin() { return iterator(this, 0U); }
    iterator end() { return iterator(this, slots_.size()); }
    const_iterator begin() const { return const_iterator(this, 0U); }
    const_iterator end() const { return const_iterator(this, slots_.size()); }

    static constexpr std::size_t npos = (std::numeric_limits<std::size_t>::max)();

private:
    // UE4's default set allocator keeps one bucket below four elements.  From
    // four onward it rounds (Num / 2 + 8) up to a power of two.  TSet rebuilds
    // every bucket chain when this value grows, walking occupied sparse slots
    // from low to high and pushing each slot at the head.  Therefore a rehash
    // changes a same-key chain to descending sparse-slot order.
    static std::uint32_t default_hash_bucket_count(const std::size_t count) {
        if (count < 4U) return 1U;
        std::uint64_t desired = count / 2U + 8U;
        std::uint64_t buckets = 1U;
        while (buckets < desired) {
            buckets <<= 1U;
            if (buckets > (std::numeric_limits<std::uint32_t>::max)()) {
                std::abort();
            }
        }
        return static_cast<std::uint32_t>(buckets);
    }

    [[nodiscard]] std::size_t active_count() const noexcept {
        return index_state_ == nullptr ? 0U : index_state_->active_count;
    }
    [[nodiscard]] std::uint32_t hash_bucket_count() const noexcept {
        return index_state_ == nullptr ? 0U : index_state_->hash_bucket_count;
    }
    void set_active_count(const std::size_t count) {
        if (count > (std::numeric_limits<std::uint32_t>::max)()) std::abort();
        ensure_index_state().active_count = static_cast<std::uint32_t>(count);
    }
    void set_hash_bucket_count(const std::uint32_t count) {
        ensure_index_state().hash_bucket_count = count;
    }
    IndexState& ensure_index_state() {
        if (index_state_ == nullptr) index_state_ = new IndexState();
        return *index_state_;
    }
    void rebuild_hash_chain_order() {
        std::uint64_t order = 0U;
        for (Slot& slot : slots_) {
            if (slot.element.has_value()) slot.hash_chain_order = ++order;
        }
        index_state_->newest_hash_chain_order = order;
    }
    void rebuild_hash_index() {
        index_state_->hash_index.clear();
        index_state_->hash_index.reserve(active_count());
        for (std::size_t index = slots_.size(); index-- > 0U;) {
            if (slots_[index].element.has_value()) {
                index_state_->hash_index[slots_[index].element->Key].push_back(index);
            }
        }
    }
    void remove_from_hash_index(const KeyType& key, const std::size_t index) {
        if (index_state_ == nullptr) std::abort();
        const auto found = index_state_->hash_index.find(key);
        if (found == index_state_->hash_index.end()) std::abort();
        auto& indices = found->second;
        const auto entry = std::find(indices.begin(), indices.end(), index);
        if (entry == indices.end()) std::abort();
        indices.erase(entry);
        if (indices.empty()) index_state_->hash_index.erase(found);
    }

    std::vector<Slot> slots_;
    std::vector<std::size_t> free_indices_;
    IndexState* index_state_ = nullptr;
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
    using Storage = TSparsePairStorage<KeyType, ValueType>;

    ValueType& Add(const KeyType& key, const ValueType& value) {
        if (auto* existing = Find(key)) {
            *existing = value;
            return *existing;
        }
        return entries_.At(entries_.Emplace(key, value))->Value;
    }
    ValueType& Add(const KeyType& key, ValueType&& value) {
        if (auto* existing = Find(key)) {
            *existing = std::move(value);
            return *existing;
        }
        return entries_.At(entries_.Emplace(key, std::move(value)))->Value;
    }
    ValueType* Find(const KeyType& key) {
        auto* found = entries_.At(find_index(key));
        return found == nullptr ? nullptr : &found->Value;
    }
    const ValueType* Find(const KeyType& key) const {
        const auto* found = entries_.At(find_index(key));
        return found == nullptr ? nullptr : &found->Value;
    }
    [[nodiscard]] ValueType FindRef(const KeyType& key) const {
        const auto* value = Find(key);
        return value == nullptr ? ValueType{} : *value;
    }
    ValueType& FindChecked(const KeyType& key) { return *Find(key); }
    const ValueType& FindChecked(const KeyType& key) const { return *Find(key); }
    [[nodiscard]] bool Contains(const KeyType& key) const { return Find(key) != nullptr; }
    int32 Remove(const KeyType& key) {
        const auto found = find_index(key);
        if (found == Storage::npos) {
            return 0;
        }
        entries_.RemoveAt(found);
        return 1;
    }
    void Reset() { entries_.Empty(); }
    void Empty() { entries_.Empty(); }
    void Reserve(const int32 count) {
        if (count > 0) {
            entries_.Reserve(static_cast<std::size_t>(count));
        }
    }
    [[nodiscard]] int32 Num() const { return entries_.Num(); }

    auto begin() noexcept { return entries_.begin(); }
    auto end() noexcept { return entries_.end(); }
    auto begin() const noexcept { return entries_.begin(); }
    auto end() const noexcept { return entries_.end(); }

private:
    std::size_t find_index(const KeyType& key) const {
        return entries_.FindHashChainIndex(key);
    }

    Storage entries_;
};

template <typename KeyType, typename ValueType>
class TMultiMap {
public:
    using ElementType = TPair<KeyType, ValueType>;
    using Storage = TSparsePairStorage<KeyType, ValueType>;

    class ConstKeyIterator {
    public:
        ConstKeyIterator(const TMultiMap& owner, const KeyType& key)
            : owner_(&owner), key_(key) {
            seek();
        }
        explicit operator bool() const { return index_ != Storage::npos; }
        ConstKeyIterator& operator++() {
            before_sequence_ = owner_->entries_.HashChainOrderAt(index_);
            seek();
            return *this;
        }
        const ValueType& Value() const { return owner_->entries_.At(index_)->Value; }
        std::size_t Index() const { return index_; }

    private:
        void seek() {
            index_ = owner_->entries_.FindHashChainIndex(key_, before_sequence_);
        }

        const TMultiMap* owner_;
        KeyType key_;
        std::uint64_t before_sequence_ =
            (std::numeric_limits<std::uint64_t>::max)();
        std::size_t index_ = Storage::npos;
    };

    void Add(const KeyType& key, const ValueType& value) {
        entries_.Emplace(key, value);
    }
    void AddUnique(const KeyType& key, const ValueType& value) {
        if (FindPair(key, value) == nullptr) {
            Add(key, value);
        }
    }
    int32 Remove(const KeyType& key, const ValueType& value) {
        int32 removed = 0;
        for (std::size_t index = 0U; index < entries_.MaxIndex(); ++index) {
            const auto* element = entries_.At(index);
            if (element != nullptr && element->Key == key && element->Value == value) {
                entries_.RemoveAt(index);
                ++removed;
            }
        }
        return removed;
    }
    ElementType* FindPair(const KeyType& key, const ValueType& value) {
        for (auto iterator = CreateConstKeyIterator(key); iterator; ++iterator) {
            if (iterator.Value() == value) return entries_.At(iterator.Index());
        }
        return nullptr;
    }
    const ElementType* FindPair(const KeyType& key, const ValueType& value) const {
        for (auto iterator = CreateConstKeyIterator(key); iterator; ++iterator) {
            if (iterator.Value() == value) return entries_.At(iterator.Index());
        }
        return nullptr;
    }
    ConstKeyIterator CreateConstKeyIterator(const KeyType& key) const {
        return ConstKeyIterator(*this, key);
    }
    void Empty() {
        entries_.Empty();
    }
    void Reserve(const int32 count) {
        if (count > 0) {
            entries_.Reserve(static_cast<std::size_t>(count));
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
