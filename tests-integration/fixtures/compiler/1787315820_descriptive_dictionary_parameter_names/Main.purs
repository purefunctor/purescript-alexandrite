module Main where

class Equal a where
  equal :: a -> a -> Boolean

class Convert source target where
  convert :: source -> target

class Available where
  available :: Boolean

data Wrapper a = Wrapper a

genericEqual :: forall value. Equal value => value -> value -> Boolean
genericEqual left right = equal left right

arrayEqual
  :: forall value
   . Equal (Array value)
  => Array value
  -> Array value
  -> Boolean
arrayEqual left right = equal left right

wrapperEqual
  :: forall value
   . Equal (Wrapper value)
  => Wrapper value
  -> Wrapper value
  -> Boolean
wrapperEqual left right = equal left right

concreteEqual :: Equal Int => Int -> Int -> Boolean
concreteEqual left right = equal left right

convertToInt :: forall value. Convert value Int => value -> Int
convertToInt value = convert value

distinctEqual
  :: forall left right
   . Equal left
  => Equal right
  => left
  -> left
  -> right
  -> right
  -> { left :: Boolean, right :: Boolean }
distinctEqual left1 left2 right1 right2 =
  { left: equal left1 left2, right: equal right1 right2 }

duplicateEqual
  :: forall value
   . Equal value
  => Equal value
  => value
  -> value
  -> Boolean
duplicateEqual left right = equal left right

parameterCollision
  :: forall value
   . Equal value
  => Boolean
  -> value
  -> value
  -> Boolean
parameterCollision equalValueDict left right =
  if equalValueDict then equal left right else false

isAvailable :: Available => Boolean
isAvailable = available
