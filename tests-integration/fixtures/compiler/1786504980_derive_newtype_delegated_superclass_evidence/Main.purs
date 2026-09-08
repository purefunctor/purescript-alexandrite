module Main where

class Parent value where
  parent :: value -> value

class Parent value <= Child value where
  child :: value -> value

class Gate

data Representation = Representation

instance Parent Representation where
  parent value = value

instance Child Representation where
  child value = value

newtype Wrapper = Wrapper Representation

instance Gate => Parent Wrapper where
  parent value = value

derive newtype instance Child Wrapper

parentFromChild :: forall value. Child value => value -> value
parentFromChild = parent

useDelegateSuperclass :: Wrapper -> Wrapper
useDelegateSuperclass = parentFromChild
