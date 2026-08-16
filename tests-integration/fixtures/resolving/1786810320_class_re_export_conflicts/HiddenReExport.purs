module HiddenReExport (module LibraryA) where

import LibraryA hiding (class Shared)
