module Main where

add :: Int -> Int -> Int
add left right = left

multiply :: Int -> Int -> Int
multiply left right = right

infixl 5 add as +
infixl 6 multiply as *

precedence :: Int
precedence = 1 + 2 * 3

leftAssociative :: Int
leftAssociative = 1 + 2 + 3

append :: Int -> Int -> Int
append left right = left

infixr 4 append as <+>

rightAssociative :: Int
rightAssociative = 1 <+> 2 <+> 3

class First :: Type -> Constraint
class First a

class Second :: Type -> Constraint
class Second a

foreign import interleaved :: forall a. First a => a -> (forall b. Second b => b -> a)

infixr 3 interleaved as <*>

implicitApplications :: forall a b. First a => Second b => a -> b -> a
implicitApplications left right = left <*> right
