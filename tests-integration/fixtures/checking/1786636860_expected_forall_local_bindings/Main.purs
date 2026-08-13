module Main where

data Unit = Unit

data Spec = Spec

data Token scope = Token

class Witness value

instance witnessValue :: Witness value

identity :: forall value. value -> value
identity value = value

modify :: forall scope. Token scope -> Token scope
modify token = token

foreign import mkEval :: forall value. Spec -> value -> value

foreign import constrainedEval
  :: forall value
   . Witness value
  => Spec
  -> value
  -> value

foreign import consumeIdentity :: (forall value. value -> value) -> Unit

foreign import leak
  :: forall result
   . (forall scope. Token scope -> result)
  -> result

validLocalEval :: Unit
validLocalEval = consumeIdentity eval
  where
  eval = mkEval Spec

validNestedLocalEval :: Unit
validNestedLocalEval = consumeIdentity outer
  where
  outer = inner
    where
    inner = mkEval Spec

escapedLocalEvidence :: Unit
escapedLocalEvidence = consumeIdentity eval
  where
  eval = constrainedEval Spec

escapedLocalAlias = escaped
  where
  escaped = leak modify

escapedEtaExpansion = leak (\token -> modify token)

escapedIdentityWrapper = identity (leak modify)

escapedArrayWrapper = [leak modify]

escapedConditionalWrapper = leak (\token -> if true then modify token else modify token)

escapedUnusedLocal :: Int
escapedUnusedLocal = 0
  where
  escaped = leak modify
