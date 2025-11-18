use std::fmt;

use crate::errors::{Result, TaError};
use crate::{Close, Next, Period, Reset};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Smoothed Simple Moving Average (SSMA).
///
/// 平滑简单移动平均：初始阶段用 SMA 逻辑计算，后续用递推公式平滑更新，无滑动窗口，更抗波动。
///
/// # Formula
/// 1. 前 N 期（count ≤ period）：SSMAₜ = (p₁ + p₂ + ... + pₜ) / t （同 SMA 逻辑）
/// 2. 第 N+1 期及以后（count > period）：SSMAₜ = (SSMAₜ₋₁ × (period - 1) + pₜ) / period
///
/// Where:
/// * _SSMAₜ_ - 第 t 期的平滑简单移动平均值
/// * _period_ - 计算周期
/// * _pₜ_ - 第 t 期的输入值
///
/// # Parameters
///
/// * _period_ - 计算周期（正整数，大于 0）
///
/// # Example
///
/// ```
/// use ta::indicators::SmoothedSimpleMovingAverage;
/// use ta::Next;
///
/// let mut ssma = SmoothedSimpleMovingAverage::new(3).unwrap();
/// assert_eq!(ssma.next(10.0), 10.0);    // 第1期：10/1
/// assert_eq!(ssma.next(11.0), 10.5);   // 第2期：(10+11)/2
/// assert_eq!(ssma.next(12.0), 11.0);   // 第3期（初始阶段结束）：(10+11+12)/3
/// assert_eq!(ssma.next(13.0).round(), 11.7); // 第4期：(11×2 +13)/3 ≈11.666...
/// assert_eq!(ssma.next(14.0).round(), 12.5); // 第5期：(11.666×2 +14)/3 ≈12.444...
/// ```
///
/// # Links
///
/// * [Smoothed Moving Average, Investopedia](https://www.investopedia.com/terms/s/smoothed-moving-average-sma.asp)
///
#[doc(alias = "SSMA")]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
pub struct SmoothedSimpleMovingAverage {
    period: usize,       // 计算周期
    current_val: f64,    // 上一期的 SSMA 值（用于递推）
    count: usize,        // 已接收的输入数据量
    sum: f64,            // 初始阶段累加和（count ≤ period 时使用）
}

impl SmoothedSimpleMovingAverage {
    pub fn new(period: usize) -> Result<Self> {
        match period {
            0 => Err(TaError::InvalidParameter), // 周期不能为0，和SMA保持一致
            _ => Ok(Self {
                period,
                current_val: 0.0, // 初始值设为0.0
                count: 0,         // 初始无数据
                sum: 0.0,         // 初始累加和为0.0
            }),
        }
    }
}

impl Period for SmoothedSimpleMovingAverage {
    fn period(&self) -> usize {
        self.period
    }
}

impl Next<f64> for SmoothedSimpleMovingAverage {
    type Output = f64;

    fn next(&mut self, input: f64) -> Self::Output {
        self.count += 1;
        self.sum += input; // 累加输入值（初始阶段用）

        // 核心逻辑：分阶段计算
        if self.count <= self.period {
            // 阶段1：前N期，复用SMA逻辑（算术平均），保证初始平滑
            self.current_val = self.sum / self.count as f64;
        } else {
            // 阶段2：第N+1期及以后，递推公式平滑更新
            self.current_val = (self.current_val * (self.period - 1) as f64 + input) / self.period as f64;
        }

        self.current_val
    }
}

// 支持 Close 类型输入（和SMA保持API兼容）
impl<T: Close> Next<&T> for SmoothedSimpleMovingAverage {
    type Output = f64;

    fn next(&mut self, input: &T) -> Self::Output {
        self.next(input.close())
    }
}

impl Reset for SmoothedSimpleMovingAverage {
    fn reset(&mut self) {
        self.current_val = 0.0; // 重置当前值
        self.count = 0;         // 重置计数
        self.sum = 0.0;         // 重置累加和
    }
}

impl Default for SmoothedSimpleMovingAverage {
    fn default() -> Self {
        Self::new(9).unwrap() // 默认周期9，和SMA保持一致
    }
}

impl fmt::Display for SmoothedSimpleMovingAverage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SSMA({})", self.period) // 显示格式：SSMA(周期)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helper::*;

    test_indicator!(SmoothedSimpleMovingAverage);

    #[test]
    fn test_new() {
        // 周期为0应报错，和SMA一致
        assert!(SmoothedSimpleMovingAverage::new(0).is_err());
        // 周期为1应正常初始化
        assert!(SmoothedSimpleMovingAverage::new(1).is_ok());
    }

    #[test]
    fn test_next() {
        let mut ssma = SmoothedSimpleMovingAverage::new(3).unwrap();

        // 阶段1：前3期（count ≤ period），和SMA结果一致
        assert_eq!(ssma.next(10.0), 10.0);    // count=1: 10/1
        assert_eq!(ssma.next(11.0), 10.5);   // count=2: (10+11)/2
        assert_eq!(ssma.next(12.0), 11.0);   // count=3: (10+11+12)/3

        // 阶段2：第4期及以后（递推公式）
        assert!((ssma.next(13.0) - 11.666666666666666).abs() < 1e-9); // (11×2 +13)/3 ≈11.666...
        assert!((ssma.next(14.0) - 12.444444444444445).abs() < 1e-9); // (11.666...×2 +14)/3 ≈12.444...
        assert!((ssma.next(15.0) - 13.296296296296296).abs() < 1e-9); // (12.444...×2 +15)/3 ≈13.296...
    }

    #[test]
    fn test_next_with_bars() {
        // 测试支持 Close 类型输入（和SMA测试风格一致）
        fn bar(close: f64) -> Bar {
            Bar::new().close(close)
        }

        let mut ssma = SmoothedSimpleMovingAverage::new(4).unwrap();
        assert_eq!(ssma.next(&bar(4.0)), 4.0);    // count=1
        assert_eq!(ssma.next(&bar(5.0)), 4.5);   // count=2
        assert_eq!(ssma.next(&bar(6.0)), 5.0);   // count=3
        assert_eq!(ssma.next(&bar(6.0)), 5.25);  // count=4（初始阶段结束）
        assert_eq!(ssma.next(&bar(6.0)), 5.75);  // count=5: (5.25×3 +6)/4=5.75
        assert_eq!(ssma.next(&bar(2.0)), 4.8125); // count=6: (5.75×3 +2)/4=4.8125
    }

    #[test]
    fn test_reset() {
        let mut ssma = SmoothedSimpleMovingAverage::new(3).unwrap();
        ssma.next(10.0);
        ssma.next(11.0);
        ssma.next(12.0);
        ssma.next(13.0);

        // 重置后应恢复初始状态
        ssma.reset();
        assert_eq!(ssma.next(99.0), 99.0); // 重置后第一期：99/1
    }

    #[test]
    fn test_default() {
        // 默认周期应为9，且初始化不报错
        let default_ssma = SmoothedSimpleMovingAverage::default();
        assert_eq!(default_ssma.period(), 9);
    }

    #[test]
    fn test_display() {
        let ssma = SmoothedSimpleMovingAverage::new(5).unwrap();
        assert_eq!(format!("{}", ssma), "SSMA(5)"); // 显示格式正确
    }

    #[test]
    fn test_period_1() {
        // 特殊情况：周期=1时，SSMA等于输入值（递推公式：(x×0 + new)/1 = new）
        let mut ssma = SmoothedSimpleMovingAverage::new(1).unwrap();
        assert_eq!(ssma.next(100.0), 100.0);
        assert_eq!(ssma.next(200.0), 200.0);
        assert_eq!(ssma.next(300.0), 300.0);
    }
}