module Main where

class Parent value

class Parent value <= Child value

data Representation = Representation

instance Parent Representation

newtype Wrapper = Wrapper Representation

instance Parent Wrapper

derive newtype instance Child Wrapper
