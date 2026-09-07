module Main where

class Parent :: Type -> Constraint
class Parent value

class Child :: Type -> Constraint
class Parent value <= Child value

instance Parent Int

instance Child Int
