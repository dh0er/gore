#pragma once

struct UAngelscriptSettings final {
    bool bErrorOnIncorrectEditorOnlyCode = false;
    bool bWarnOnDivergentComparisonOperatorOverloads = false;
    bool bWarnOnImplicitSignedUnsignedConversion = false;
    bool bWarnOnIncrementDecrementInComplexExpression = false;
    bool bWarnOnUnusedReturnValueForConstMethods = false;
    bool bErrorWhenUsingInvalidWorldContext = false;
};
