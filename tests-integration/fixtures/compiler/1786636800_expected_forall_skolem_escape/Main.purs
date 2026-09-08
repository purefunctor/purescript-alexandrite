module Main where

data Unit = Unit

data Token scope = Token

data Effect value = Effect value

data Proxy value = Proxy

data Map key value = Map

class Witness scope

class EscapingEvidence scope result | result -> scope

identity :: forall value. value -> value
identity value = value

modify :: forall scope. Token scope -> Token scope
modify token = token

foreign import consumeIdentity :: (forall value. value -> value) -> Unit

foreign import discardValue :: forall value. value -> Unit

foreign import leak
  :: forall result
   . (forall scope. Token scope -> result)
  -> result

foreign import mutate
  :: forall discarded
   . (forall scope. Token scope -> Effect discarded)
  -> Unit

foreign import mutateConstrained
  :: (forall scope. Witness scope => Token scope -> Effect Unit)
  -> Unit

foreign import poke :: forall scope. Token scope -> Effect Unit

foreign import pokeReturningToken
  :: forall scope
   . Token scope
  -> Effect (Token scope)

foreign import pokeConstrained
  :: forall scope
   . Witness scope
  => Token scope
  -> Effect Unit

foreign import pokeWithoutWitness
  :: forall scope result
   . EscapingEvidence scope result
  => Token scope
  -> Proxy result

foreign import mutateMap
  :: forall key value discarded
   . Witness key
  => (forall scope. Token scope -> Effect discarded)
  -> Map key value
  -> Map key value

foreign import pokeMap
  :: forall key value scope
   . Witness key
  => key
  -> value
  -> Token scope
  -> Effect (Token scope)

validRankTwoCallback :: (forall value. value -> value) -> Unit
validRankTwoCallback callback = consumeIdentity callback

validDirectPolymorphicExpression :: Unit
validDirectPolymorphicExpression = consumeIdentity identity

validDiscardedHigherRankResult :: Unit
validDiscardedHigherRankResult = mutate poke

validDiscardedScopedHigherRankResult :: Unit
validDiscardedScopedHigherRankResult = mutate pokeReturningToken

validDiscardedConstrainedHigherRankResult :: Unit
validDiscardedConstrainedHigherRankResult = mutateConstrained pokeConstrained

validConstrainedTypeErasingCallback
  :: forall key value
   . Witness key
  => key
  -> value
  -> Map key value
  -> Map key value
validConstrainedTypeErasingCallback key value = mutateMap (pokeMap key value)

escapedResult = leak modify

escapedBinder = \callback -> consumeIdentity callback

escapedNestedUse = identity (leak modify)

escapedDiscardedOuterArgument _ = leak modify

escapedDiscardedArgument :: Unit
escapedDiscardedArgument = discardValue (leak modify)

escapedEvidenceOnly = leak pokeWithoutWitness
