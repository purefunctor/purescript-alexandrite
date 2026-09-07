module Main where

identity :: forall a. a -> a
identity value = value

inlineGlobal :: Boolean -> Boolean
inlineGlobal value = identity value

inlineProperty :: { value :: Boolean } -> Boolean
inlineProperty record = identity record.value

inlineLiteral :: Boolean -> Boolean
inlineLiteral condition = if condition then true else false

inlineClosure :: Boolean -> Boolean
inlineClosure value = (\item -> item) value

inlineCall :: (Boolean -> Boolean) -> Boolean -> Boolean
inlineCall function value = function value

inlineArray :: Boolean -> Array Boolean
inlineArray value = [value]

inlineRecord :: Boolean -> { value :: Boolean }
inlineRecord value = { value }

inlineCapturedClosure :: Boolean -> Boolean -> Boolean
inlineCapturedClosure captured = \_ -> captured

inlineAlias :: Boolean -> Boolean
inlineAlias value =
  let alias = value
  in alias

inlineRepeatedAlias :: Boolean -> { first :: Boolean, second :: Boolean }
inlineRepeatedAlias value =
  let alias = value
  in { first: alias, second: alias }

inlineSingleUseProperty :: { value :: Boolean } -> Boolean
inlineSingleUseProperty record =
  let value = record.value
  in identity value

keepMultiUseClosure
  :: Boolean
  -> { first :: Boolean -> Boolean, second :: Boolean -> Boolean }
keepMultiUseClosure captured =
  let closure = \_ -> captured
  in { first: closure, second: closure }

keepMultiUse :: { value :: Boolean } -> { first :: Boolean, second :: Boolean }
keepMultiUse record =
  let value = record.value
  in { first: value, second: value }

inlineAcrossCall
  :: { value :: Boolean }
  -> (Boolean -> Boolean)
  -> { projected :: Boolean, called :: Boolean }
inlineAcrossCall record function =
  { projected: record.value, called: function true }

inlineOrderedCalls
  :: (Boolean -> Boolean)
  -> (Boolean -> Boolean)
  -> { first :: Boolean, second :: Boolean }
inlineOrderedCalls first second =
  { first: first true, second: second false }

keepReorderedCalls
  :: (Boolean -> Boolean)
  -> (Boolean -> Boolean)
  -> { first :: Boolean, second :: Boolean }
keepReorderedCalls first second =
  let
    firstResult = first true
    secondResult = second false
  in
    { first: secondResult, second: firstResult }

keepMultiUseCall
  :: (Boolean -> Boolean)
  -> Boolean
  -> { first :: Boolean, second :: Boolean }
keepMultiUseCall function value =
  let result = function value
  in { first: result, second: result }

keepCallBeforeBranch :: Boolean -> (Boolean -> Boolean) -> Boolean -> Boolean
keepCallBeforeBranch condition function value =
  let result = function value
  in if condition then result else value

keepTestCall :: (Boolean -> Array Boolean) -> Boolean -> Boolean
keepTestCall function value = case function value of
  [] -> true
  _ -> false
