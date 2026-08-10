module Main where

class First :: Type -> Constraint
class First a

class Second :: Type -> Constraint
class Second a

foreign import interleaved :: forall a. First a => a -> (forall b. Second b => b -> a)

applyInterleaved :: forall a b. First a => Second b => a -> b -> a
applyInterleaved first second = interleaved first second

layered :: forall a. First a => (forall b. Second b => a -> b -> a)
layered first _ = first
