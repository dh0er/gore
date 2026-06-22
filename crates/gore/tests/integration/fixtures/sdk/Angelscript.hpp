// Class Angelscript.UGothicObjectDefinition
// Size: 0x50 (Inherited: 0x28)
class UGothicObjectDefinition : public UObject
{
public:
    int32_t m_Name; // 0x28(0x04)[placeholder type for fixture]
};

// Class Angelscript.UItemDefinition
// Size: 0x2F0 (Inherited: 0x50)
class UItemDefinition : public UGothicObjectDefinition
{
public:
    int32_t m_Value; // 0x50(0x04)
    int32_t m_MaxStack; // 0x54(0x04)
    float m_Weight; // 0x58(0x04)
    float m_Mass; // 0x5C(0x04)
    bool m_Buoyancy; // 0x60(0x01)
};

// Class Angelscript.ItFo_Apple
// Size: 0x320 (Inherited: 0x2F0)
class ItFo_Apple : public UItemDefinition
{
public:
};

// Enum Angelscript.EItemQuality
enum class EItemQuality : uint8_t
{
    Low = 0,
    Medium = 1,
    High = 2,
};
