-- | Main module documentation.
module Main where

import Library (LibraryType)

-- | A documented data type.
data Choice :: Type
data Choice = FirstChoice | SecondChoice

-- | A documented newtype.
newtype Identity :: Type -> Type
newtype Identity a = Identity a

-- | A documented synonym.
type IdentityAlias :: Type -> Type
type IdentityAlias a = Identity a

-- | A documented superclass.
class Label :: Type -> Constraint
class Label a where
  -- | A documented superclass member.
  label :: a -> String

-- | A documented class.
class Render :: Type -> Type -> Constraint
class Label a <= Render a b | a -> b where
  -- | A documented member.
  render :: a -> b -> String

-- | A documented multi-parameter class.
class Convert :: Type -> Type -> Type -> Constraint
class Convert a b c | a b -> c, c -> a b

-- | A documented superclass instance.
instance labelChoice :: Label Choice where
  label _ = "choice"

-- | A documented instance.
instance renderChoice :: Render Choice String where
  render _ value = value

-- | A documented value.
answer :: LibraryType -> Choice
answer _ = FirstChoice
