module Main where

class Default a where
  default :: a

useDefault :: forall a. Default a => a
useDefault = default

class First :: Type -> Constraint
class First a

class Second :: Type -> Constraint
class Second a

foreign import interleaved :: forall a. First a => (forall b. Second b => a -> b)

useInterleaved :: forall a b. First a => Second b => a -> b
useInterleaved = interleaved
