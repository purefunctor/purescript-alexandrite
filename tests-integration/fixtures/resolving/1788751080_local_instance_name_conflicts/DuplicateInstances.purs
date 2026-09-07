module DuplicateInstances where

class Class :: Type -> Constraint
class Class a

instance duplicate :: Class Int
instance duplicate :: Class String

newtype First = First Int
newtype Second = Second Int
newtype Third = Third Int

derive newtype instance duplicate :: Class First
derive newtype instance derived :: Class Second
instance derived :: Class Boolean
derive newtype instance derived :: Class Third
