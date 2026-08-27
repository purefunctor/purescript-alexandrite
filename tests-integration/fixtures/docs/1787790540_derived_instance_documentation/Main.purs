module Main where

import Remote (Remote)
import Wrapping (class Wrap)

-- | A local class used by a derived instance.
class LocalWrap a

-- | A local type used by derived instances.
data Box = Box

-- | A derived instance associated with both local declarations.
derive instance localWrapBox :: LocalWrap Box

-- | A derived instance for an imported class and type.
derive instance wrapRemote :: Wrap Remote
