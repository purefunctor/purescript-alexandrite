module Main where

foreign import data Mutable :: Type -> Type
foreign import data Action :: Type -> Type -> Type
foreign import data Values :: Type -> Type
foreign import data Token :: Type -> Type
foreign import data Box :: Type -> Type
foreign import data FixedRegion :: Type

class Marker :: Type -> Constraint
class Marker region

class ResultConstraint :: Type -> Constraint
class ResultConstraint result

instance resultConstraintMutable :: ResultConstraint (Mutable region)

foreign import unsafeValue :: forall value. value

foreign import pureM
  :: forall (monad :: Type -> Type) value
   . value
  -> monad value

foreign import bind
  :: forall (monad :: Type -> Type) value result
   . monad value
  -> (value -> monad result)
  -> monad result

foreign import map
  :: forall (functor :: Type -> Type) value result
   . (value -> result)
  -> functor value
  -> functor result

foreign import apply
  :: forall (apply_ :: Type -> Type) value result
   . apply_ (value -> result)
  -> apply_ value
  -> apply_ result

foreign import mutate
  :: forall value result
   . (forall region. Mutable region -> Action region result)
  -> value

foreign import modify
  :: forall region
   . Mutable region
  -> Action region (Mutable region)

foreign import modifyFunction
  :: forall region
   . Mutable region
  -> Action region (Int -> Mutable region)

foreign import modifyRecord
  :: forall region
   . Mutable region
  -> Action region { value :: Mutable region }

foreign import modifyArray
  :: forall region
   . Mutable region
  -> Action region (Array (Mutable region))

foreign import modifySynonym
  :: forall region
   . Mutable region
  -> Action region (Synonym (Mutable region))

foreign import modifyBox
  :: forall region
   . Mutable region
  -> Action region (Box (Mutable region))

foreign import poke
  :: forall value region
   . String
  -> value
  -> Mutable region
  -> Action region (Mutable region)

foreign import delete
  :: forall region
   . String
  -> Mutable region
  -> Action region (Mutable region)

foreign import values :: Values Int

foreign import foldAction
  :: forall region value accumulator
   . (accumulator -> value -> Action region accumulator)
  -> accumulator
  -> Values value
  -> Action region accumulator

mutateConstrained
  :: forall value result
   . (forall region. Marker region => Mutable region -> Action region result)
  -> value
mutateConstrained = unsafeValue

constrainedModify
  :: forall region
   . Marker region
  => Mutable region
  -> Action region (Mutable region)
constrainedModify = unsafeValue

mutateWithResultConstraint
  :: forall value result
   . ResultConstraint result
  => (forall region. Mutable region -> Action region result)
  -> value
mutateWithResultConstraint = unsafeValue

nestedMutate
  :: forall value result
   . (forall region. Marker region => Mutable region -> (forall token. Token token -> Action region result))
  -> value
nestedMutate = unsafeValue

nestedModify
  :: forall region
   . Marker region
  => Mutable region
  -> forall token
   . Token token
  -> Action region (Mutable region)
nestedModify = unsafeValue

foreign import leak
  :: forall result
   . (forall region. Mutable region -> Action region result)
  -> result

leakConstrained
  :: forall result
   . (forall region. Marker region => Mutable region -> Action region result)
  -> result
leakConstrained = unsafeValue

foreign import discard :: forall value. value -> Int

foreign import consume
  :: forall result
   . (forall region. Mutable region -> Action region result)
  -> result
  -> Int

foreign import consumeFunction
  :: forall result
   . (forall region. Mutable region -> Action region result)
  -> result
  -> Int

foreign import consumeRecord
  :: forall result
   . (forall region. Mutable region -> Action region result)
  -> { value :: result }
  -> Int

foreign import consumeArray
  :: forall result
   . (forall region. Mutable region -> Action region result)
  -> Array result
  -> Int

foreign import consumeSynonym
  :: forall result
   . (forall region. Mutable region -> Action region result)
  -> Synonym result
  -> Int

foreign import captureFunction
  :: forall result
   . (forall region. Mutable region -> Action region result)
  -> (Int -> result -> Int)

foreign import captureVisible
  :: forall result
   . (forall region. Mutable region -> Action region result)
  -> (forall @token. Token token -> result -> Int)

foreign import captureVisibleArgument
  :: forall result
   . (forall region. Mutable region -> Action region result)
  -> (forall @token. (result -> token) -> Int)

foreign import consumeIndependent
  :: forall left right
   . (forall region. Mutable region -> Action region left)
  -> (forall region. Mutable region -> Action region right)
  -> right
  -> Int

foreign import monomorphicModify
  :: Mutable FixedRegion
  -> Action FixedRegion (Mutable FixedRegion)

data Wrapper value = Wrapper value

applyValue :: forall value result. (value -> result) -> value -> result
applyValue function value = function value

infixr 0 applyValue as <|

identity :: forall value. value -> value
identity value = value

type Synonym value =
  { nested :: { value :: value } }

positiveModify :: Int
positiveModify = mutate modify

positivePoke :: Int
positivePoke = mutate (poke "key" 1)

positiveDelete :: Int
positiveDelete = mutate (delete "key")

positiveFold :: Int
positiveFold = mutate \mutable ->
  foldAction (\accumulator value -> poke "key" value accumulator) mutable values

positiveConstrained :: Int
positiveConstrained = mutateConstrained constrainedModify

positiveResultConstraint :: Int
positiveResultConstraint = mutateWithResultConstraint modify

positiveNested :: Int
positiveNested = nestedMutate nestedModify

positiveDo :: Int
positiveDo = mutate \mutable -> do
  value <- pureM mutable
  pureM value

positiveAdo :: Int
positiveAdo = mutate \mutable -> ado
  value <- pureM mutable
  in value

positiveDoLet :: Int
positiveDoLet = mutate \mutable -> do
  first <- pureM mutable
  let second = first
  third <- pureM second
  pureM third

positiveAdoLet :: Int
positiveAdoLet = mutate \mutable -> ado
  first <- pureM mutable
  _ <- pureM mutable
  let second = first
  in second

escaped = leak modify

discarded :: Int
discarded = discard (leak modify)

binderEscape = \value -> consume modify value

wrappedEscape = Wrapper (leak modify)

recordEscape = { value: leak modify }

constrainedEscape = leakConstrained constrainedModify

functionEscape :: Int
functionEscape = consumeFunction modifyFunction (\_ -> unsafeValue)

recordLiteralEscape :: Int
recordLiteralEscape = consumeRecord modifyRecord { value: unsafeValue }

arrayLiteralEscape :: Int
arrayLiteralEscape = consumeArray modifyArray [ unsafeValue ]

ifEscape :: Int
ifEscape = consumeFunction modify (if true then unsafeValue else unsafeValue)

caseEscape :: Int
caseEscape = consumeFunction modify (case true of
  true -> unsafeValue
  false -> unsafeValue)

inferredCaseEscape = case true of
  true -> leak modify
  false -> unsafeValue

inferredGuardedEscape
  | true = leak modify
  | false = unsafeValue

letEscape :: Int
letEscape = consumeFunction modify (let value = unsafeValue in value)

synonymEscape :: Int
synonymEscape = consumeSynonym modifySynonym { nested: { value: unsafeValue } }

multiStepEscape :: Int
multiStepEscape = captureFunction modify 0 unsafeValue

parenthesizedMultiStepEscape :: Int
parenthesizedMultiStepEscape = (captureFunction modify 0) unsafeValue

visibleTypeApplicationEscape :: Int
visibleTypeApplicationEscape = captureVisible modify @Int unsafeValue unsafeValue

visibleTypeArgumentEscape :: Int
visibleTypeArgumentEscape = captureVisibleArgument modify @_ identity

operatorEscape :: Int
operatorEscape = captureFunction modify 0 <| unsafeValue

sectionEscape :: Int
sectionEscape = consumeFunction modifyFunction (unsafeValue <| _)

doEscape :: Int
doEscape = consume modifyBox do
  value <- pureM unsafeValue
  pureM value

adoEscape :: Int
adoEscape = consume modifyBox ado
  value <- pureM unsafeValue
  in value

independentOwnerEscape :: Int
independentOwnerEscape = consumeIndependent modify modify unsafeValue

monomorphicCallback :: Int
monomorphicCallback = mutate monomorphicModify
