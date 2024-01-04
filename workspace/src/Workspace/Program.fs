open StreamJsonRpc
open FSharp.Collections.ParallelSeq

let total =
    [ "1"; "2"; "3" ]
    |> PSeq.map (fun x -> int x)
    |> PSeq.map (fun x -> x * 2)
    |> PSeq.toList
    |> List.fold (fun acc x -> acc + x) 0

printfn $"Hello {total} times!"
