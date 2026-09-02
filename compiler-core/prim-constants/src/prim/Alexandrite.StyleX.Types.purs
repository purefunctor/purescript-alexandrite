module Alexandrite.StyleX.Types
  ( Angle
  , Color
  , Url
  , Image
  , Integer
  , LengthPercentage
  , Length
  , Percentage
  , Number
  , Resolution
  , Time
  , TransformFunction
  , TransformList
  , angle
  , color
  , url
  , image
  , integer
  , lengthPercentage
  , length
  , percentage
  , number
  , resolution
  , time
  , transformFunction
  , transformList
  ) where

import Alexandrite.StyleX (TypedValue)

data Angle
data Color
data Url
data Image
data Integer
data LengthPercentage
data Length
data Percentage
data Number
data Resolution
data Time
data TransformFunction
data TransformList

foreign import angle :: forall value. value -> TypedValue Angle value
foreign import color :: forall value. value -> TypedValue Color value
foreign import url :: forall value. value -> TypedValue Url value
foreign import image :: forall value. value -> TypedValue Image value
foreign import integer :: forall value. value -> TypedValue Integer value
foreign import lengthPercentage
  :: forall value. value -> TypedValue LengthPercentage value
foreign import length :: forall value. value -> TypedValue Length value
foreign import percentage :: forall value. value -> TypedValue Percentage value
foreign import number :: forall value. value -> TypedValue Number value
foreign import resolution :: forall value. value -> TypedValue Resolution value
foreign import time :: forall value. value -> TypedValue Time value
foreign import transformFunction :: forall value. value -> TypedValue TransformFunction value
foreign import transformList :: forall value. value -> TypedValue TransformList value
