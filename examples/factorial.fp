(
  ($x x)                       $force
  (force cswap $_ force)       $if
  ($f $t $c $fn ^f ^t ^c fn)   $endif

  ; Y-Combinator
  ($f
    ($x (^x x) f)
    ($x (^x x) f)
    force
  ) $Y

  ; rec: syntax sugar for applying the Y-Combinator
  ($g (^g Y)) $rec

  ; factorial
  ($self $n
    ^if (^n 0 eq)
      (1)
      (^n 1 - self ^n *)
    endif
  ) rec $factorial

  'factorial-recursive print
  5 factorial print

  ; alternative tail recursive approach
  ($self $n $acc
    ^if (^n 0 eq)
      (acc)
      (^n acc * ^n 1 - self)
    endif
  ) rec $--factorial
  ($x 1 x --factorial) $factorial

  'factorial-tail-recursive print
  5 factorial print
)
