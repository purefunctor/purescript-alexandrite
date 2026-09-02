module Alexandrite.StyleX.When
  ( ancestor
  , ancestorMarker
  , descendant
  , descendantMarker
  , siblingBefore
  , siblingBeforeMarker
  , siblingAfter
  , siblingAfterMarker
  , anySibling
  , anySiblingMarker
  ) where

import Alexandrite.StyleX (ConditionalCase, Marker)

foreign import ancestor :: forall value. String -> value -> ConditionalCase value

foreign import ancestorMarker
  :: forall value
   . String
  -> Marker
  -> value
  -> ConditionalCase value

foreign import descendant :: forall value. String -> value -> ConditionalCase value

foreign import descendantMarker
  :: forall value
   . String
  -> Marker
  -> value
  -> ConditionalCase value

foreign import siblingBefore :: forall value. String -> value -> ConditionalCase value

foreign import siblingBeforeMarker
  :: forall value
   . String
  -> Marker
  -> value
  -> ConditionalCase value

foreign import siblingAfter :: forall value. String -> value -> ConditionalCase value

foreign import siblingAfterMarker
  :: forall value
   . String
  -> Marker
  -> value
  -> ConditionalCase value

foreign import anySibling :: forall value. String -> value -> ConditionalCase value

foreign import anySiblingMarker
  :: forall value
   . String
  -> Marker
  -> value
  -> ConditionalCase value
