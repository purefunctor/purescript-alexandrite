module Main where

class Class :: Type -> Constraint
class Class a

instance classInt :: Class Int
foreign import classInt :: Int

test :: Int
test = classInt
