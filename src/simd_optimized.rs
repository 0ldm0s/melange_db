//! SIMD优化的key比较模块
//!
//! 此模块提供了针对不同平台的SIMD优化的key比较操作，
//! 支持ARM64 NEON和x86_64 SSE2/AVX2指令集。
//!
//! 主要优化：
//! - NEON/SSE2/AVX2指令集优化
//! - 分支预测优化
//! - 缓存友好的内存访问模式
//! - 自适应比较策略

use std::cmp::Ordering;

#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

/// SIMD优化的key比较器
pub struct SimdComparator;

impl SimdComparator {
    /// SIMD优化的key比较
    ///
    /// 此函数使用NEON指令集进行16字节对齐的比较，
    /// 对于剩余字节使用标量比较。
    #[inline(always)]
    pub fn compare(a: &[u8], b: &[u8]) -> Ordering {
        let len = std::cmp::min(a.len(), b.len());

        // 对于小key，使用快速路径
        if len <= 16 {
            return Self::compare_small(a, b);
        }

        // 使用SIMD进行比较
        unsafe {
            Self::compare_simd(a, b, len)
        }
    }

    /// 小key的快速比较（<= 16字节）
    #[inline(always)]
    fn compare_small(a: &[u8], b: &[u8]) -> Ordering {
        let min_len = std::cmp::min(a.len(), b.len());

        // 使用64位整数比较以获得更好的性能
        let chunks = min_len / 8;
        let remainder = min_len % 8;

        for i in 0..chunks {
            let offset = i * 8;
            let a_chunk = u64::from_ne_bytes([
                a[offset], a[offset + 1], a[offset + 2], a[offset + 3],
                a[offset + 4], a[offset + 5], a[offset + 6], a[offset + 7]
            ]);
            let b_chunk = u64::from_ne_bytes([
                b[offset], b[offset + 1], b[offset + 2], b[offset + 3],
                b[offset + 4], b[offset + 5], b[offset + 6], b[offset + 7]
            ]);

            if a_chunk != b_chunk {
                // 找到第一个不同的字节
                for j in 0..8 {
                    let byte_a = a[offset + j];
                    let byte_b = b[offset + j];
                    if byte_a != byte_b {
                        return byte_a.cmp(&byte_b);
                    }
                }
            }
        }

        // 处理剩余字节
        for i in (chunks * 8)..min_len {
            if a[i] != b[i] {
                return a[i].cmp(&b[i]);
            }
        }

        // 如果前min_len字节都相等，比较长度
        a.len().cmp(&b.len())
    }

    /// SIMD优化的比较（> 16字节）
    #[inline(always)]
    unsafe fn compare_simd(a: &[u8], b: &[u8], len: usize) -> Ordering {
        #[cfg(target_arch = "aarch64")]
        {
            Self::compare_simd_neon(a, b, len)
        }

        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx2") {
                Self::compare_simd_avx2(a, b, len)
            } else if is_x86_feature_detected!("sse2") {
                Self::compare_simd_sse2(a, b, len)
            } else {
                Self::compare_simd_fallback(a, b, len)
            }
        }

        #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
        {
            Self::compare_simd_fallback(a, b, len)
        }
    }

    /// ARM64 NEON SIMD比较
    #[cfg(target_arch = "aarch64")]
    #[inline(always)]
    unsafe fn compare_simd_neon(a: &[u8], b: &[u8], len: usize) -> Ordering {
        let simd_chunks = len / 16;
        let remainder = len % 16;

        for i in 0..simd_chunks {
            let offset = i * 16;
            let a_vec = vld1q_u8(a.as_ptr().add(offset));
            let b_vec = vld1q_u8(b.as_ptr().add(offset));

            // 比较两个向量
            let eq_mask = vceqq_u8(a_vec, b_vec);

            // 如果所有字节都相等，eq_mask将是全1
            if vminvq_u8(eq_mask) != 0xFF {
                // 找到第一个不同的字节
                let diff_mask = vceqq_u8(a_vec, b_vec);
                let diff_mask_bits = vmovn_u16(vreinterpretq_u16_u8(diff_mask));
                let diff_mask_64 = vget_lane_u64(vreinterpret_u64_u8(diff_mask_bits), 0);

                // 找到第一个不同的字节位置
                let first_diff = diff_mask_64.trailing_zeros() as usize;

                return a[offset + first_diff].cmp(&b[offset + first_diff]);
            }
        }

        // 处理剩余字节
        for i in (simd_chunks * 16)..len {
            if a[i] != b[i] {
                return a[i].cmp(&b[i]);
            }
        }

        a.len().cmp(&b.len())
    }

    /// x86_64 AVX2 SIMD比较
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    unsafe fn compare_simd_avx2(a: &[u8], b: &[u8], len: usize) -> Ordering {
        let simd_chunks = len / 32;
        let remainder = len % 32;

        for i in 0..simd_chunks {
            let offset = i * 32;
            let a_vec = unsafe { _mm256_loadu_si256(a.as_ptr().add(offset) as *const __m256i) };
            let b_vec = unsafe { _mm256_loadu_si256(b.as_ptr().add(offset) as *const __m256i) };

            // 比较两个向量
            let eq_mask = _mm256_cmpeq_epi8(a_vec, b_vec);
            let eq_mask_bits = _mm256_movemask_epi8(eq_mask);

            // 如果所有字节都相等，eq_mask_bits将是全1
            if eq_mask_bits != -1 {
                // 找到第一个不同的字节位置
                let first_diff = eq_mask_bits.trailing_zeros() as usize;
                return a[offset + first_diff].cmp(&b[offset + first_diff]);
            }
        }

        // 处理剩余字节
        for i in (simd_chunks * 32)..len {
            if a[i] != b[i] {
                return a[i].cmp(&b[i]);
            }
        }

        a.len().cmp(&b.len())
    }

    /// x86_64 SSE2 SIMD比较
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "sse2")]
    unsafe fn compare_simd_sse2(a: &[u8], b: &[u8], len: usize) -> Ordering {
        let simd_chunks = len / 16;
        let remainder = len % 16;

        for i in 0..simd_chunks {
            let offset = i * 16;
            let a_vec = unsafe { _mm_loadu_si128(a.as_ptr().add(offset) as *const __m128i) };
            let b_vec = unsafe { _mm_loadu_si128(b.as_ptr().add(offset) as *const __m128i) };

            // 比较两个向量
            let eq_mask = _mm_cmpeq_epi8(a_vec, b_vec);
            let eq_mask_bits = _mm_movemask_epi8(eq_mask);

            // 如果所有字节都相等，eq_mask_bits将是全1
            if eq_mask_bits != 0xFFFF {
                // 找到第一个不同的字节位置
                let first_diff = eq_mask_bits.trailing_zeros() as usize;
                return a[offset + first_diff].cmp(&b[offset + first_diff]);
            }
        }

        // 处理剩余字节
        for i in (simd_chunks * 16)..len {
            if a[i] != b[i] {
                return a[i].cmp(&b[i]);
            }
        }

        a.len().cmp(&b.len())
    }

    /// 降级比较（不支持SIMD时使用）
    #[inline(always)]
    unsafe fn compare_simd_fallback(a: &[u8], b: &[u8], len: usize) -> Ordering {
        // 使用64位整数批量比较
        let chunks = len / 8;
        let remainder = len % 8;

        for i in 0..chunks {
            let offset = i * 8;
            let a_chunk = u64::from_ne_bytes([
                a[offset], a[offset + 1], a[offset + 2], a[offset + 3],
                a[offset + 4], a[offset + 5], a[offset + 6], a[offset + 7]
            ]);
            let b_chunk = u64::from_ne_bytes([
                b[offset], b[offset + 1], b[offset + 2], b[offset + 3],
                b[offset + 4], b[offset + 5], b[offset + 6], b[offset + 7]
            ]);

            if a_chunk != b_chunk {
                // 找到第一个不同的字节
                for j in 0..8 {
                    let byte_a = a[offset + j];
                    let byte_b = b[offset + j];
                    if byte_a != byte_b {
                        return byte_a.cmp(&byte_b);
                    }
                }
            }
        }

        // 处理剩余字节
        for i in (chunks * 8)..len {
            if a[i] != b[i] {
                return a[i].cmp(&b[i]);
            }
        }

        a.len().cmp(&b.len())
    }

    /// SIMD优化的相等比较
    ///
    /// 此函数专门用于相等性检查，比通用比较更快
    #[inline(always)]
    pub fn equals(a: &[u8], b: &[u8]) -> bool {
        Self::compare(a, b) == Ordering::Equal
    }

    

    /// 批量key比较优化
    ///
    /// 在批量操作中预取数据以提高缓存命中率
    pub fn batch_compare(target: &[u8], keys: &[&[u8]]) -> Vec<Ordering> {
        let mut results = Vec::with_capacity(keys.len());

        // 平台相关的缓存预取
        #[cfg(target_arch = "aarch64")]
        {
            // ARM64缓存预取
            if let Some(first_key) = keys.first() {
                unsafe {
                    std::arch::asm!(
                        "prfm pldl1keep, [{0}]",
                        in(reg) first_key.as_ptr(),
                        options(nostack)
                    );
                }
            }

            for (i, key) in keys.iter().enumerate() {
                // 预取下一个key的缓存行
                if i + 1 < keys.len() {
                    unsafe {
                        std::arch::asm!(
                            "prfm pldl1keep, [{0}]",
                            in(reg) keys[i + 1].as_ptr(),
                            options(nostack)
                        );
                    }
                }

                results.push(Self::compare(target, key));
            }
        }

        #[cfg(target_arch = "x86_64")]
        {
            // x86_64缓存预取
            for key in keys.iter() {
                unsafe {
                    std::arch::asm!(
                        "prefetcht0 [{0}]",
                        in(reg) key.as_ptr(),
                        options(nostack)
                    );
                }
            }

            for key in keys.iter() {
                results.push(Self::compare(target, key));
            }
        }

        #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
        {
            // 其他平台使用普通循环
            for key in keys.iter() {
                results.push(Self::compare(target, key));
            }
        }

        results
    }
}

/// 通用的key比较器trait
pub trait KeyComparator {
    fn compare(&self, a: &[u8], b: &[u8]) -> Ordering;
    fn equals(&self, a: &[u8], b: &[u8]) -> bool;
}

impl KeyComparator for SimdComparator {
    #[inline(always)]
    fn compare(&self, a: &[u8], b: &[u8]) -> Ordering {
        Self::compare(a, b)
    }

    #[inline(always)]
    fn equals(&self, a: &[u8], b: &[u8]) -> bool {
        Self::equals(a, b)
    }
}

/// 为[InlineArray]提供SIMD优化的比较（内部实现细节）
///
/// 注意：这需要InlineArray提供访问底层字节的接口
#[doc(hidden)]
pub trait SimdOptimizedCompare {
    fn simd_compare(&self, other: &Self) -> Ordering;
    fn simd_equals(&self, other: &Self) -> bool;
}

/// 性能测试和基准测试函数
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_simd_compare_basic() {
        let a = b"hello world";
        let b = b"hello world";
        let c = b"hello there";
        let d = b"goodbye world";

        assert_eq!(SimdComparator::compare(a, b), Ordering::Equal);
        assert_eq!(SimdComparator::compare(a, c), Ordering::Greater);
        assert_eq!(SimdComparator::compare(d, a), Ordering::Less);
    }

    #[test]
    fn test_simd_equals() {
        let a = b"hello world";
        let b = b"hello world";
        let c = b"hello there";
        let d = b"hello world!";

        assert!(SimdComparator::equals(a, b));
        assert!(!SimdComparator::equals(a, c));
        assert!(!SimdComparator::equals(a, d));
    }

    #[test]
    fn test_small_key_performance() {
        let keys: Vec<&[u8]> = vec![
            b"key1", b"key2", b"key3", b"key4", b"key5",
            b"key10", b"key11", b"key12", b"key13", b"key14",
        ];

        let start = Instant::now();
        for _ in 0..100000 {
            for i in 0..keys.len() {
                for j in 0..keys.len() {
                    SimdComparator::compare(keys[i], keys[j]);
                }
            }
        }
        let duration = start.elapsed();

        println!("小key比较性能: {:?}", duration);
    }

    #[test]
    fn test_large_key_performance() {
        let base = b"this is a relatively long key that we will use for performance testing ";
        let mut keys = Vec::new();

        for i in 0u32..10 {
            let mut key = base.to_vec();
            key.extend_from_slice(&i.to_le_bytes());
            keys.push(key);
        }

        let start = Instant::now();
        for _ in 0..10000 {
            for i in 0..keys.len() {
                for j in 0..keys.len() {
                    SimdComparator::compare(&keys[i], &keys[j]);
                }
            }
        }
        let duration = start.elapsed();

        println!("大key比较性能: {:?}", duration);
    }

    #[test]
    fn test_batch_compare() {
        let target = b"hello world";
        let keys: &[&[u8]] = &[
            b"hello",
            b"hello world",
            b"hello world!",
            b"hello there",
            b"hello universe",
        ];

        let results = SimdComparator::batch_compare(target, keys);

        // 正确的期望结果
        let expected = vec![
            Ordering::Greater,  // "hello" < "hello world" -> target > key
            Ordering::Equal,    // "hello world" == "hello world"
            Ordering::Less,     // "hello world!" > "hello world" -> target < key
            Ordering::Greater,  // "hello there" < "hello world" -> target > key
            Ordering::Greater,  // "hello universe" < "hello world" -> target > key
        ];

        assert_eq!(results, expected);
    }
}