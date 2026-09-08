module Main where

data Missing = Missing

class Parent :: Type -> Constraint
class Parent value

class Child :: Type -> Constraint
class Parent value <= Child value

instance Child Missing
