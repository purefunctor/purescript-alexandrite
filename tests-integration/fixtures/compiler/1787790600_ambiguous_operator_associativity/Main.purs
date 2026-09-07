module Main where

nonAssociative left right = left

infix 5 nonAssociative as <=>

singleNonAssociative :: Int
singleNonAssociative = 1 <=> 2

ambiguousNonAssociative :: Int
ambiguousNonAssociative = 1 <=> 2 <=> 3

leftAssociative left right = left
rightAssociative left right = right

infixl 6 leftAssociative as <+>
infixr 6 rightAssociative as <*>

infixr 7 rightAssociative as <**>

validLeftAssociative :: Int
validLeftAssociative = 1 <+> 2 <+> 3

validRightAssociative :: Int
validRightAssociative = 1 <*> 2 <*> 3

validMixedPrecedence :: Int
validMixedPrecedence = 1 <+> 2 <**> 3

ambiguousMixedAssociativity :: Int
ambiguousMixedAssociativity = 1 <+> 2 <*> 3
