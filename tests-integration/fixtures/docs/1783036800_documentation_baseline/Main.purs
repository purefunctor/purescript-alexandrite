-- | Main module documentation.
module Main where

import Library (LibraryType)

-- | A documented data type.
data Choice = FirstChoice | SecondChoice

-- | A documented synonym.
type ChoiceAlias = Choice

-- | A documented class.
class Render a where
  -- | A documented member.
  render :: a -> String

-- | A documented instance.
instance renderChoice :: Render Choice where
  render _ = "choice"

-- | A documented value.
answer :: LibraryType -> Choice
answer _ = FirstChoice
