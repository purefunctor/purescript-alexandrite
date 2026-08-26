module Main where

import Data.Eq as Eq

compareTwice :: forall a. Eq.Eq a => a -> a -> Boolean
compareTwice left right =
  if Eq.eq left right then Eq.eq right left else false

compareIntsTwice :: Int -> Int -> Boolean
compareIntsTwice left right =
  if Eq.eq left right then Eq.eq right left else false

compareArraysTwice :: Array Int -> Array Int -> Boolean
compareArraysTwice left right =
  if Eq.eq left right then Eq.eq right left else false

compareArraysOnce :: Array Int -> Array Int -> Boolean
compareArraysOnce left right = Eq.eq left right

compareGenericArraysTwice :: forall a. Eq.Eq a => Array a -> Array a -> Boolean
compareGenericArraysTwice left right =
  if Eq.eq left right then Eq.eq right left else false

compareNestedArraysTwice
  :: Array Int
  -> Array Int
  -> Array (Array Int)
  -> Array (Array Int)
  -> Boolean
compareNestedArraysTwice left right nestedLeft nestedRight =
  if Eq.eq nestedLeft nestedRight then Eq.eq nestedRight nestedLeft else Eq.eq left right

distinctGivens
  :: forall a b
   . Eq.Eq a
  => Eq.Eq b
  => Array a
  -> Array a
  -> Array b
  -> Array b
  -> Boolean
distinctGivens leftA rightA leftB rightB =
  if Eq.eq leftA rightA then Eq.eq rightA leftA
  else if Eq.eq leftB rightB then Eq.eq rightB leftB
  else false

distinctSubgoals
  :: Array Int
  -> Array Int
  -> Array Boolean
  -> Array Boolean
  -> Boolean
distinctSubgoals leftInt rightInt leftBoolean rightBoolean =
  if Eq.eq leftInt rightInt then Eq.eq rightInt leftInt
  else if Eq.eq leftBoolean rightBoolean then Eq.eq rightBoolean leftBoolean
  else false

compareArraysThrice :: Array Int -> Array Int -> Boolean
compareArraysThrice left right =
  if Eq.eq left right
  then if Eq.eq right left then Eq.eq left right else false
  else false

compareNestedArraysWhole
  :: Array (Array Int)
  -> Array (Array Int)
  -> Boolean
compareNestedArraysWhole left right =
  if Eq.eq left right then Eq.eq right left else false

compareSuperclassArraysTwice
  :: forall a
   . Eq.Ordered a
  => Array a
  -> Array a
  -> Boolean
compareSuperclassArraysTwice left right =
  if Eq.eq left right then Eq.eq right left else false

compareSuperclassTwice
  :: forall a
   . Eq.Ordered a
  => a
  -> a
  -> Boolean
compareSuperclassTwice left right =
  if Eq.eq left right then Eq.eq right left else false

lambdaScope :: Array Int -> Array Int -> Boolean
lambdaScope left right =
  if Eq.eq left right
  then (\lambdaLeft lambdaRight ->
    if Eq.eq lambdaLeft lambdaRight then Eq.eq lambdaRight lambdaLeft else false
  ) left right
  else false

whereIsolation :: Array Int -> Array Int -> Boolean
whereIsolation left right =
  if helper left right then Eq.eq left right else false
  where
  helper helperLeft helperRight = Eq.eq helperLeft helperRight

equationScope :: Boolean -> Array Int -> Array Int -> Boolean
equationScope true left right = Eq.eq left right
equationScope false left right = Eq.eq right left

firstComparison :: Boolean
firstComparison = Eq.eq 1 2

secondComparison :: Boolean
secondComparison = Eq.eq 3 4

data Recursive

instance Eq.Eq Recursive where
  eq left right = Eq.eq left right

compareRecursiveTwice :: Recursive -> Recursive -> Boolean
compareRecursiveTwice left right =
  if Eq.eq left right then Eq.eq right left else false
