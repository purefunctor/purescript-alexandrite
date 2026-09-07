module HiddenForeignFirst (test) where

class Class :: Type -> Constraint
class Class a

foreign import classInt :: Int
instance classInt :: Class Int

test :: Int
test = classInt
