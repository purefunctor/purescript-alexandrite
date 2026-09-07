module Main where

class Eq :: Type -> Constraint
class Eq a where
  show :: a -> String

record :: { show :: forall a. Eq a => a -> String }
record = { show }

class First :: Type -> Constraint
class First a where
  interleaved :: forall b. Second b => a -> b

class Second :: Type -> Constraint
class Second b

interleavedRecord
  :: { interleaved :: forall a. First a => (forall b. Second b => a -> b) }
interleavedRecord = { interleaved }
