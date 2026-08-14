module Main where

class InferredTypeEq a b (result :: Boolean) | a b -> result
--                   $ $  $

instance InferredTypeEq a a True
--                      $ $
else instance InferredTypeEq a b False
--                           $ $

class ExplicitTypeEq :: forall kind. kind -> kind -> Boolean -> Constraint
class ExplicitTypeEq a b result | a b -> result
--                   $ $ $

instance ExplicitTypeEq a a True
--                      $ $
else instance ExplicitTypeEq a b False
--                           $ $
