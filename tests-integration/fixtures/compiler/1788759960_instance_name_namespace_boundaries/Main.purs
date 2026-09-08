module Main where

import Library (imported)

class Class :: Type -> Constraint
class Class a

instance imported :: Class Int
instance Class String
instance local :: Class Boolean

foreign import dictionary :: Int
foreign import classString :: Int

test :: Int
test = imported

shadow :: Int -> Int
shadow local = local

record :: { local :: Int }
record = { local: dictionary }
