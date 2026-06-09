//! 书签管理
//!
//! 在文稿中标记位置，支持快速跳转。

use crate::script::Marker;

/// 书签管理器
#[derive(Debug, Default)]
pub struct MarkerManager {
    markers: Vec<Marker>,
}

impl MarkerManager {
    pub fn new() -> Self {
        Self {
            markers: Vec::new(),
        }
    }

    /// 添加书签
    pub fn add(&mut self, name: String, offset: usize) {
        // 如果同名已存在，更新位置
        if let Some(existing) = self.markers.iter_mut().find(|m| m.name == name) {
            existing.offset = offset;
            return;
        }
        self.markers.push(Marker::new(name, offset));
        // 按偏移排序
        self.markers.sort_by_key(|m| m.offset);
    }

    /// 删除书签
    pub fn remove(&mut self, name: &str) -> Option<Marker> {
        if let Some(pos) = self.markers.iter().position(|m| m.name == name) {
            Some(self.markers.remove(pos))
        } else {
            None
        }
    }

    /// 查找指定偏移之后的下一个书签
    pub fn next_after(&self, offset: usize) -> Option<&Marker> {
        self.markers.iter().find(|m| m.offset > offset)
    }

    /// 查找指定偏移之前的上一个书签
    pub fn prev_before(&self, offset: usize) -> Option<&Marker> {
        self.markers.iter().rev().find(|m| m.offset < offset)
    }

    /// 列出所有书签
    pub fn list(&self) -> &[Marker] {
        &self.markers
    }

    /// 书签数量
    pub fn len(&self) -> usize {
        self.markers.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.markers.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_list() {
        let mut mm = MarkerManager::new();
        mm.add("intro".into(), 0);
        mm.add("chapter1".into(), 500);
        mm.add("chapter2".into(), 1000);

        assert_eq!(mm.len(), 3);
        assert_eq!(mm.list()[0].name, "intro");
        assert_eq!(mm.list()[1].name, "chapter1");
        assert_eq!(mm.list()[2].name, "chapter2");
    }

    #[test]
    fn test_add_duplicate_updates() {
        let mut mm = MarkerManager::new();
        mm.add("mark".into(), 100);
        mm.add("mark".into(), 200);

        assert_eq!(mm.len(), 1);
        assert_eq!(mm.list()[0].offset, 200);
    }

    #[test]
    fn test_remove() {
        let mut mm = MarkerManager::new();
        mm.add("a".into(), 0);
        mm.add("b".into(), 10);

        let removed = mm.remove("a");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().name, "a");
        assert_eq!(mm.len(), 1);
    }

    #[test]
    fn test_remove_nonexistent() {
        let mut mm = MarkerManager::new();
        assert!(mm.remove("nope").is_none());
    }

    #[test]
    fn test_next_after() {
        let mut mm = MarkerManager::new();
        mm.add("a".into(), 100);
        mm.add("b".into(), 200);
        mm.add("c".into(), 300);

        assert_eq!(mm.next_after(150).unwrap().name, "b");
        assert_eq!(mm.next_after(250).unwrap().name, "c");
        assert!(mm.next_after(350).is_none());
    }

    #[test]
    fn test_prev_before() {
        let mut mm = MarkerManager::new();
        mm.add("a".into(), 100);
        mm.add("b".into(), 200);
        mm.add("c".into(), 300);

        assert_eq!(mm.prev_before(250).unwrap().name, "b");
        assert_eq!(mm.prev_before(150).unwrap().name, "a");
        assert!(mm.prev_before(50).is_none());
    }

    #[test]
    fn test_is_empty() {
        let mm = MarkerManager::new();
        assert!(mm.is_empty());
        assert_eq!(mm.len(), 0);
    }
}
