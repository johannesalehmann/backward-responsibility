# Peg Solitaire

To analyse optimistic responsibility, run

    ./target/release/bw-responsibility --prism-model experiments/peg_solitaire/peg_solitaire.prism -c "experiments/peg_solitaire/peg_solitaire.ce" -b "sbar" -v o

You should get the following responsibilities. 

| State                                                                                                                                                             | Responsibility |
|-------------------------------------------------------------------------------------------------------------------------------------------------------------------|----------------|
| (p00=false, p10=false, p11=false, p20=true,  p21=false, p22=true,  p30=true, p31=true,  p32=true,  p33=true, p40=true, p41=true,  p42=true,  p43=true,  p44=true) |     0.12500000 |
| (p00=false, p10=false, p11=false, p20=true,  p21=true,  p22=false, p30=true, p31=false, p32=false, p33=true, p40=true, p41=false, p42=false, p43=true,  p44=true) |     0.12500000 |
| (p00=false, p10=false, p11=false, p20=true,  p21=true,  p22=false, p30=true, p31=false, p32=false, p33=true, p40=true, p41=true,  p42=true,  p43=false, p44=true) |     0.12500000 |
| (p00=false, p10=false, p11=false, p20=true,  p21=true,  p22=true,  p30=true, p31=false, p32=true,  p33=true, p40=true, p41=false, p42=true,  p43=true,  p44=true) |     0.12500000 |
| (p00=false, p10=false, p11=false, p20=true,  p21=true,  p22=true,  p30=true, p31=false, p32=true,  p33=true, p40=true, p41=true,  p42=false, p43=false, p44=true) |     0.12500000 |
| (p00=false, p10=true,  p11=true,  p20=true,  p21=true,  p22=true,  p30=true, p31=true,  p32=true,  p33=true, p40=true, p41=true,  p42=true,  p43=true,  p44=true) |     0.12500000 |
| (p00=true,  p10=false, p11=true,  p20=false, p21=true,  p22=true,  p30=true, p31=true,  p32=true,  p33=true, p40=true, p41=true,  p42=true,  p43=true,  p44=true) |     0.12500000 |
| (p00=true,  p10=false, p11=true,  p20=true,  p21=false, p22=false, p30=true, p31=true,  p32=true,  p33=true, p40=true, p41=true,  p42=true,  p43=true,  p44=true) |     0.12500000 |

All states up to the one in Figure 2b have responsibility 0.125, whereas all following states don't have positive responsibility.

Note that the numbering scheme differs slightly from the paper, with pij corresponding to the jth hole in the ith row:

| Internal Name | Name in Paper |
|---------------|---------------|
|  p00          | 1             |
|  p10          | 2             |
|  p11          | 3             |
|  p20          | 4             |
|  p21          | 5             |
|  p22          | 6             |
|  p30          | 7             |
|  p31          | 8             |
|  p32          | 9             |
|  p33          | 10            |
|  p40          | 11            |
|  p41          | 12            |
|  p42          | 13            |
|  p43          | 14            |
|  p44          | 15            |
