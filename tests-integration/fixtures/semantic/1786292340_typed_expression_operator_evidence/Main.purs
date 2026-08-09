module Main where

class Supplied a where
  supplied :: a

consume :: (Supplied Int => Int) -> Int
consume _ = 0

identity :: forall a. a -> a
identity value = value

apply :: forall a b. (a -> b) -> a -> b
apply function argument = function argument

infixr 0 apply as <|

throughOperator :: Int
throughOperator = consume <| (supplied :: Supplied Int => Int)

throughNestedOperator :: Int
throughNestedOperator = consume <| identity <| (supplied :: Supplied Int => Int)

polymorphicResult :: Int -> Int -> forall a. a -> a
polymorphicResult _ _ value = value

infixr 0 polymorphicResult as <@>

throughPolymorphicResult :: forall a. a -> a
throughPolymorphicResult = 0 <@> 0

constrainedResult :: Int -> Int -> (Supplied Int => Int)
constrainedResult _ _ = supplied

infixr 0 constrainedResult as <#>

throughConstrainedResult :: Supplied Int => Int
throughConstrainedResult = 0 <#> 0
