// MIT License
//
// Copyright (c) 2026 worksoup <https://github.com/worksoup/>
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

//! 提供只支持“重置到开头”（rewind）操作的 I/O trait 及其适配器。
//!
//! 标准库中的 [`Seek`] trait 要求实现者能够支持任意位置的定位（通过 [`SeekFrom`](std::io::SeekFrom)），
//! 这对于某些流式 reader（如网络流、压缩流、加密流等）来说过于严格。它们通常只能
//! 重置到初始状态，而无法高效地跳转到任意位置。
//!
//! 本模块定义了 [`Rewind`] trait，它只要求实现 `try_rewind` 方法，明确表达了“只能重置”
//! 的能力，避免与 `Seek` 的强契约产生冲突。同时为 [`Cursor`] 和任意 `Seek` 类型
//! 提供了便捷的实现适配。

use std::io::{Cursor, Seek};

/// 标记 trait，表示类型可以安全地进行“无条件重置”。
///
/// 当一个类型实现了 [`Rewind`] 且其 `try_rewind` 方法保证永不失败时，
/// 可以实现此空 trait。这样，该类型就可以使用 [`Rewind::rewind`] 和
/// [`Rewind::rebuild`] 这两个便捷方法，它们会直接 `unwrap` 结果，避免
/// 不必要的 `Result` 处理。
///
/// # 安全性
///
/// 实现此 trait 的类型必须确保其 `try_rewind` 方法在所有情况下都成功，
/// 不会返回 `Err`。如果违反了这一约定，调用 `rewind` 或 `rebuild` 将会 panic。
pub trait RewindEasily {}
/// 一个只能将 I/O 对象重置到开头（rewind）的 trait。
///
/// 该 trait 与 [`Seek`] 不同，它不承诺任意跳转的能力，只要求实现 `try_rewind` 方法，
/// 将内部指针或状态重置到初始位置（相当于 [`SeekFrom::Start(0)`](std::io::SeekFrom)）。
///
/// # 示例
///
/// ```
/// use std::io::{Cursor, Read};
/// use reader_ext::Rewind;
///
/// let mut cursor = Cursor::new(vec![1, 2, 3]);
/// // 读取一些数据后重置
/// cursor.read_exact(&mut [0; 2]).unwrap();
/// cursor.try_rewind().unwrap();
/// let mut buf = [0; 2];
/// cursor.read_exact(&mut buf).unwrap();
/// assert_eq!(buf, [1, 2]);
/// // 现在可以重新从头读取
/// ```
pub trait Rewind {
    /// 消费当前对象，返回一个重置到开头的对象。
    ///
    /// 这是一个便利方法，先调用 [`try_rewind`](Self::try_rewind) 重置，然后将自身返回。
    /// 如果重置失败，则错误被传播，原对象被丢弃。
    ///
    /// # 示例
    ///
    /// ```
    /// use std::io::{Cursor, Read};
    /// use reader_ext::Rewind;
    ///
    /// let mut cursor = Cursor::new(vec![1, 2, 3]);
    /// cursor.read_exact(&mut [0; 2]).unwrap();
    /// let mut cursor = cursor.try_rebuild().unwrap();  // 重置并取得所有权
    /// let mut buf = [0; 2];
    /// cursor.read_exact(&mut buf).unwrap();
    /// assert_eq!(buf, [1, 2]);
    /// // cursor 已处于初始位置
    /// ```
    #[inline]
    fn try_rebuild(mut self) -> std::io::Result<Self>
    where
        Self: Sized,
    {
        self.try_rewind()?;
        Ok(self)
    }

    /// 重置当前 I/O 对象到初始位置。
    ///
    /// 该方法的行为应与 `Seek::seek(SeekFrom::Start(0))` 一致，但只要求支持
    /// 重置操作，不要求支持任意 seek。
    ///
    /// # 错误
    ///
    /// 如果重置操作失败（例如底层 reader 不可重置），则应返回一个 I/O 错误。
    fn try_rewind(&mut self) -> std::io::Result<()>;

    /// 重置当前 I/O 对象到初始位置，如果失败则 panic。
    ///
    /// 此方法仅在 `Self` 同时实现了 [`RewindEasily`] 时可用，因为只有
    /// 保证 `try_rewind` 永不失败的类型才能安全地调用此方法而不处理错误。
    ///
    /// # Panics
    ///
    /// 如果内部的 `try_rewind` 返回错误，此方法将 panic。
    #[inline]
    fn rewind(&mut self)
    where
        Self: RewindEasily,
    {
        self.try_rewind().unwrap();
    }

    /// 消费当前对象，返回一个重置到开头的对象，如果失败则 panic。
    ///
    /// 此方法仅在 `Self` 同时实现了 [`RewindEasily`] 时可用，它先调用
    /// [`try_rewind`](Self::try_rewind) 重置，然后返回自身。由于 `Self`
    /// 保证重置永远不会失败，因此可以直接 `unwrap`。
    ///
    /// # Panics
    ///
    /// 如果内部的 `try_rewind` 返回错误，此方法将 panic。
    #[inline]
    fn rebuild(self) -> Self
    where
        Self: RewindEasily + Sized,
    {
        self.try_rebuild().unwrap()
    }
}

/// 为 [`Cursor`] 实现 [`Rewind`] trait。
impl<T> Rewind for Cursor<T>
where
    T: AsRef<[u8]>,
{
    #[inline]
    fn try_rewind(&mut self) -> std::io::Result<()> {
        Seek::rewind(self)
    }
}

/// 一个包装器，为任何实现了 [`Seek`] 的类型实现 [`Rewind`] trait。
///
/// 注意：此包装器仅将 `try_rewind` 转发给内部的 [`Seek::rewind`] 方法，因此它要求
/// 内部的 [`Seek`] 实现确实支持重置到开头。如果内部的 `Seek` 实现不支持（例如
/// 某些自定义类型可能不支持 [`SeekFrom::Start(0)`](std::io::SeekFrom)），则 `try_rewind` 仍可能失败。
///
/// 通常，您可以直接使用此包装器来适配那些已经实现了 `Seek` 但您希望以 `Rewind`
/// 方式使用的类型。
///
/// # 示例
///
/// ```
/// use std::io::{Cursor, Seek};
/// use reader_ext::{Rewind, Rewinder};
///
/// let cursor = Cursor::new(vec![1, 2, 3]);
/// let rewinder = Rewinder::new(cursor);  // 包装成 Rewind
/// // 现在可以调用 rewinder.try_rewind() 或 rewinder.try_rebuild()
/// ```
pub struct Rewinder<T: Seek>(T);

impl<T: Seek> Rewinder<T> {
    /// 创建一个新的 [`Rewinder`]，包装给定的实现了 [`Seek`] 的对象。
    #[inline]
    pub fn new(inner: T) -> Self {
        Rewinder(inner)
    }
}

impl<T: Seek> Rewind for Rewinder<T> {
    #[inline]
    fn try_rewind(&mut self) -> std::io::Result<()> {
        self.0.rewind()
    }
}
