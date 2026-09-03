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

class StringValue :: Type -> Constraint

instance StringValue String

class NumberValue :: Type -> Constraint

instance NumberValue Int
instance NumberValue Prim.Number

class StringOrNumberValue :: Type -> Constraint

instance StringOrNumberValue String
instance StringOrNumberValue Int
instance StringOrNumberValue Prim.Number

foreign import angle
  :: forall value. StringOrNumberValue value => value -> TypedValue Angle value

foreign import color :: forall value. StringValue value => value -> TypedValue Color value
foreign import url :: forall value. StringValue value => value -> TypedValue Url value
foreign import image :: forall value. StringValue value => value -> TypedValue Image value
foreign import integer :: forall value. NumberValue value => value -> TypedValue Integer value
foreign import lengthPercentage
  :: forall value
   . StringOrNumberValue value
  => value
  -> TypedValue LengthPercentage value

foreign import length
  :: forall value. StringOrNumberValue value => value -> TypedValue Length value

foreign import percentage
  :: forall value. StringOrNumberValue value => value -> TypedValue Percentage value

foreign import number :: forall value. NumberValue value => value -> TypedValue Number value
foreign import resolution
  :: forall value. StringValue value => value -> TypedValue Resolution value

foreign import time
  :: forall value. StringOrNumberValue value => value -> TypedValue Time value

foreign import transformFunction
  :: forall value. StringValue value => value -> TypedValue TransformFunction value

foreign import transformList
  :: forall value. StringValue value => value -> TypedValue TransformList value
