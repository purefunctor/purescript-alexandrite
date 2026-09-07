module Derived where

class Class :: Type -> Constraint
class Class a
instance base :: Class Int

newtype First = First Int
newtype Second = Second Int

derive newtype instance first :: Class First
foreign import first :: Int

foreign import second :: Int
derive newtype instance second :: Class Second
