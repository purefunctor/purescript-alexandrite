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

keepCall :: (Boolean -> Boolean) -> Boolean -> Boolean
keepCall function value = function value

keepArray :: Boolean -> Array Boolean
keepArray value = [value]

keepRecord :: Boolean -> { value :: Boolean }
keepRecord value = { value }

keepCapturedClosure :: Boolean -> Boolean -> Boolean
keepCapturedClosure captured = \_ -> captured

keepMultiUse :: { value :: Boolean } -> { first :: Boolean, second :: Boolean }
keepMultiUse record =
  let value = record.value
  in { first: value, second: value }

keepAcrossCall
  :: { value :: Boolean }
  -> (Boolean -> Boolean)
  -> { projected :: Boolean, called :: Boolean }
keepAcrossCall record function =
  { projected: record.value, called: function true }
