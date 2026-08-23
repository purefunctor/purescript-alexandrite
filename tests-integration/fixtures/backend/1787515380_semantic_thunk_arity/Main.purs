module Main where

data Unit = Unit

ordinaryUnit :: Unit -> Unit
ordinaryUnit value = value

ordinaryUnitCall :: Unit
ordinaryUnitCall = ordinaryUnit Unit

class Parent a where
  parent :: a -> a

class Parent a <= Child a where
  child :: a -> a

instance Parent Int where
  parent value = value

instance childArray :: Parent (Array a) => Child (Array a) where
  child value = value

useSuperclass :: forall a. Child a => a -> a
useSuperclass value = parent value
