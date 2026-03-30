This is a binary to compute loan schedules in compliance with the IFRS. 

This is one of my first rust projects, shared under the AGPLv3 license for educational purposes only.

Features :
1) Clean room implementation based on IFRS 9 requirements and the Newton-Raphson method
2) Near Zero-Copy , Struct-of-Arrays implementation

Benchmarks

### Environment
- **CPU:** Intel Core i7-14700HX (Raptor Lake)
- **OS:** Linux Mint 22.3 (Cinnamon)
- **RAM:** 16GB DDR5 @ 5600MT/s
- **Allocator:** jemalloc (Linux) / mimalloc (Windows)

### Single-Core Results
Tested using `taskset -c 7` (Physical P-Core) with `POLARS_MAX_THREADS=1` and `RAYON_NUM_THREADS=1`.

Average Execution time  776.115µs, on a loan of 20 years with monthly installments (This excludes IO), and 2 tranches
IO took 6ms seconds in the benchmarked run.

 Performance counter stats for 'target/release/irr':

       8,27,33,816      task-clock                          0.983 CPUs utilized
             3,729      context-switches                   45.072 K/sec
                 0      cpu-migrations                      0.000 /sec
             2,002      page-faults                        24.198 K/sec
      91,68,55,811      cpu_core/instructions/              2.16  insn per cycle
      42,41,01,260      cpu_core/cycles/                    5.126 GHz
      13,41,90,712      cpu_core/branches/                  1.622 G/sec
         19,26,535      cpu_core/branch-misses/             1.44% of all branches
                                                             28.8 %  tma_backend_bound
                                                              8.4 %  tma_bad_speculation
                                                             27.2 %  tma_frontend_bound
                                                             35.6 %  tma_retiring

       0.084169926 seconds time elapsed
       0.069678000 seconds user
       0.013935000 seconds sys
