module Main where

class Parent :: forall k. k -> Constraint
class Parent value

class Child :: forall k. k -> Constraint
class Parent value <= Child value

instance Parent value => Child value
