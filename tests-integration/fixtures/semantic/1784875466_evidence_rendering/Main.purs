module Main where

class Parent :: Type -> Constraint
class Parent value

class Parent value <= Child value

instance parentInt :: Parent Int

instance childInt :: Parent Int => Child Int

foreign import useParent :: forall value. Parent value => value -> value

foreign import useChild :: forall value. Child value => value -> value

instanceEvidence :: Int
instanceEvidence = useChild 1

superclassEvidence :: forall value. Child value => value -> value
superclassEvidence = useParent
