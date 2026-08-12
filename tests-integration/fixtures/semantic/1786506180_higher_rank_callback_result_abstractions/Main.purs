module Main where

foreign import data Mutable :: Type -> Type
foreign import data Action :: Type -> Type -> Type
foreign import data Token :: Type -> Type

class Marker :: Type -> Constraint
class Marker region

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

positiveModify :: Int
positiveModify = mutate modify

positiveConstrained :: Int
positiveConstrained = mutateConstrained constrainedModify

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
