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

-- | A documented class.
class Render :: Type -> Constraint
class Render a where
  -- | A documented member.
  render :: a -> String

-- | A documented instance.
instance renderChoice :: Render Choice where
  render _ = "choice"

-- | A documented value.
answer :: LibraryType -> Choice
answer _ = FirstChoice
