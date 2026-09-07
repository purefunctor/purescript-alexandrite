module Values where

class Class :: Type -> Constraint
class Class a

instance classInt :: Class Int
classInt :: Int
classInt = 42

classString :: String
classString = "value"
instance classString :: Class String
