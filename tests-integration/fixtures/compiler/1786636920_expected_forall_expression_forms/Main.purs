module Main where

data Token scope = Token

data Effect value = Effect value

identity :: forall value. value -> value
identity value = value

modify :: forall scope. Token scope -> Token scope
modify token = token

foreign import leak
  :: forall result
   . (forall scope. Token scope -> result)
  -> result

foreign import pure :: forall value. value -> Effect value

foreign import bind
  :: forall value result
   . Effect value
  -> (value -> Effect result)
  -> Effect result

foreign import discard
  :: forall value result
   . Effect value
  -> (value -> Effect result)
  -> Effect result

foreign import map
  :: forall value result
   . (value -> result)
  -> Effect value
  -> Effect result

foreign import apply
  :: forall value result
   . Effect (value -> result)
  -> Effect value
  -> Effect result

applyValue :: forall value result. value -> (value -> result) -> result
applyValue value function = function value

infixl 1 applyValue as #

escapedOperator = leak modify # identity

escapedSection = (leak modify # _)

escapedDo = do
  token <- pure (leak modify)
  pure token

escapedAdo = ado
  token <- pure (leak modify)
  in token

escapedRecord = { token: leak modify }

escapedRecordUpdate record = record { token = leak modify }

escapedPatternGuard value
  | token <- leak modify = value

escapedCase value = case value of
  token -> { token: leak modify }
