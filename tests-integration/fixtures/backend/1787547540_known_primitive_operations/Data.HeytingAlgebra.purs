module Data.HeytingAlgebra where

class HeytingAlgebra value where
  not :: value -> value

instance heytingAlgebraBoolean :: HeytingAlgebra Boolean where
  not value = if value then false else true
