(
  ;; Core: Helpers
  ($x ^x ^x)                       $dup
  ($x x)                           $force
  ($x)                             $drop
  ($x $y ^x ^y)                    $swap
  ($x $y $z ^x ^z ^y)              $rot
  (0 swap - -)                     $+
  (force cswap drop force)         $if
  ($a $b $c $d ^a ^b ^c ^d force)  $endif
  ($cond 't '() ^cond if)          $not
  (0 swap - -)                     $+
  ($fn $arg (^arg fn))             $partial

  ;; These are just delimiters to make code easier to look at.
  ()                               $[
  ()                               $]

  ;; Core: Recursion
  ($f
    ($x (^x x) f)
    ($x (^x x) f)
    force
  ) $Y

  (^Y partial) $rec

  ;; TCO recursive range function
  ($self $acc $start $end
    ^if (^start ^end eq)
      (acc end cons)
      (end 1 -
       start
       acc end cons
       self)
    endif
  ) rec $--range

  ;; Proper range function
  '() ^--range partial $range

  ;; Recursive reduce function
  ($self $fn $init $list
    ^if (list '() eq)
      (init)
      (list cdr
       list car init fn
       ^fn
       self)
    endif
  ) rec $reduce

  [ 1 18 << ] $end

  end print
  '=> print

    end 0 range
    0 (+) reduce
  print
)