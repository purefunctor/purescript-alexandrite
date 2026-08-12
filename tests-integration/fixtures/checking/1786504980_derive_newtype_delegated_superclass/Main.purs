module Main where

class Parent :: (Type -> Type) -> Constraint
class Parent constructor

class Parent constructor <= Child constructor

class Gate :: (Type -> Type) -> Constraint
class Gate constructor

data Representation :: (Type -> Type) -> Type -> Type
data Representation parameter value = Representation

instance Parent (Representation parameter)
instance Child (Representation parameter)

newtype Wrapper :: (Type -> Type) -> Type -> Type
newtype Wrapper parameter value = Wrapper (Representation parameter value)

instance Gate parameter => Parent (Wrapper parameter)

derive newtype instance Child (Wrapper parameter)
