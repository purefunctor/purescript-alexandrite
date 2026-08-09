module Main where

class Eq :: Type -> Constraint
class Eq a where
  show :: a -> String

evidenceDoesNotEscape
  :: forall a
   . { scoped :: Eq a => a -> String
     , leaked :: a -> String
     }
evidenceDoesNotEscape = { scoped: show, leaked: show }

foreign import consume
  :: forall result
   . { value :: forall a. a -> result }
  -> result

typeDoesNotEscape = consume { value: \value -> value }
