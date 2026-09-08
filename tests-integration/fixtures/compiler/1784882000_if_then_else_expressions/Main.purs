module Main where

class Predicate a where
  predicate :: a -> Boolean

class Default value where
  defaultValue :: value

checked :: Boolean -> Int
checked condition = if condition then 1 else 2

inferred condition = if condition then "yes" else "no"

constrained :: forall value. Predicate value => value -> Int
constrained value = if predicate value then 1 else 2

constrainedBranches :: forall value. Default value => Boolean -> value
constrainedBranches condition = if condition then defaultValue else defaultValue

higherRank :: Boolean -> (forall value. value -> value)
higherRank condition = if condition then identity else identity

nested first second = if first then if second then 1 else 2 else 3

asArgument condition = identity (if condition then 1 else 2)

identity :: forall value. value -> value
identity value = value
