module Direct (module Origin, module Again, append) where

import Origin as Again
import Origin
  ( Option(Just)
  , Wrapped(..)
  , await
  , class Measure
  , foreignValue
  , measure
  , type (:*:)
  , visible
  , (<>)
  ) as Origin

append :: Int
append = 99
