module Main where

import Data.Unit (Unit)
import Safe.Coerce (coerce)

data Map :: Type -> Type -> Type
data Map key value = Map

newtype Set :: Type -> Type
newtype Set value = Set (Map value Unit)

foreign import filterKeys
  :: forall key
   . (key -> Boolean)
  -> (forall value. Map key value -> Map key value)

filter :: forall value. (value -> Boolean) -> Set value -> Set value
filter = coerce (filterKeys :: _ -> Map value Unit -> _)
