module Members where

class Class a where
  classInt :: a

instance classInt :: Class Int where
  classInt = 42

instance classString :: Other String where
  classString = "value"

class Other a where
  classString :: a
