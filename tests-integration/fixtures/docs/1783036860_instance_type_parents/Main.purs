module Main where

-- | Box data type.
data Box = Box

-- | Pair data type.
data Pair a b = Pair a b

-- | Alias synonym.
type Alias = Box

-- | Inspect class.
class Inspect a where
  inspect :: a -> String

-- | Instance mentioning a data type, synonym, and repeated data type.
instance inspectPairAliasBox :: Inspect (Pair Alias Box) where
  inspect _ = "pair"

-- | Instance mentioning the same local type more than once.
instance inspectPairBoxBox :: Inspect (Pair Box Box) where
  inspect _ = "repeated"
