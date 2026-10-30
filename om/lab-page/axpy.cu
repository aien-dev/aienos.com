// The same kernel in CUDA C: y[i] = a*x[i] + y[i], one element per thread.
#include <cstdio>
#include <cuda_runtime.h>

__global__ void axpy(long long a, const long long *x, long long *y, long long n) {
  long long i = (long long)blockIdx.x * blockDim.x + threadIdx.x;
  if (i < n) y[i] = a * x[i] + y[i];
}

int main() {
  const long long n = 1 << 20, a = 3;
  long long *x, *y;
  cudaMallocManaged(&x, n * sizeof *x);
  cudaMallocManaged(&y, n * sizeof *y);
  for (long long i = 0; i < n; i++) { x[i] = i; y[i] = 2 * i + 1; }
  axpy<<<(n + 255) / 256, 256>>>(a, x, y, n);
  cudaDeviceSynchronize();
  long long s = 0;
  for (long long i = 0; i < n; i++) s += y[i];
  printf("%lld\n", s);
  cudaFree(x);
  cudaFree(y);
  return 0;
}
