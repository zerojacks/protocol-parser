//! AFN (应用功能码) 定义
//!
//! AFN 分为两大类：
//! - 集中器功能码（DI3=E8H）
//! - 采集器功能码（DI3=EAH）
//!
//! 不同节点角色使用不同的 AFN 表

/// 应用功能码 AFN
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Afn {
    /// 00H: 确认/否认
    AckNack,
    /// 01H: 复位
    Reset,
    /// 02H: 设置参数
    SetParam,
    /// 03H: 查询参数
    QueryParam,
    /// 04H: 数据转发
    DataForward,
    /// 05H: 控制命令
    Control,
    /// 06H: 启动从节点搜索
    StartNodeSearch,
    /// 07H: 查询文件传输失败节点
    QueryFileTransferFailure,
    /// 08H: 路由查询
    RouteQuery,
    /// 09H: 文件传输
    FileTransfer,
    /// 21H: 通知存储从节点档案（采集器专用）
    NotifyStoreNodeArchive,
    /// 其他未定义的功能码
    Other(u8),
}

impl Afn {
    /// 从字节解析 AFN
    pub fn from_byte(byte: u8) -> Self {
        match byte {
            0x00 => Afn::AckNack,
            0x01 => Afn::Reset,
            0x02 => Afn::SetParam,
            0x03 => Afn::QueryParam,
            0x04 => Afn::DataForward,
            0x05 => Afn::Control,
            0x06 => Afn::StartNodeSearch,
            0x07 => Afn::QueryFileTransferFailure,
            0x08 => Afn::RouteQuery,
            0x09 => Afn::FileTransfer,
            0x21 => Afn::NotifyStoreNodeArchive,
            other => Afn::Other(other),
        }
    }

    /// 转换为字节
    pub fn to_byte(&self) -> u8 {
        match self {
            Afn::AckNack => 0x00,
            Afn::Reset => 0x01,
            Afn::SetParam => 0x02,
            Afn::QueryParam => 0x03,
            Afn::DataForward => 0x04,
            Afn::Control => 0x05,
            Afn::StartNodeSearch => 0x06,
            Afn::QueryFileTransferFailure => 0x07,
            Afn::RouteQuery => 0x08,
            Afn::FileTransfer => 0x09,
            Afn::NotifyStoreNodeArchive => 0x21,
            Afn::Other(byte) => *byte,
        }
    }

    /// 获取功能码描述
    pub fn description(&self) -> &'static str {
        match self {
            Afn::AckNack => "确认/否认",
            Afn::Reset => "复位",
            Afn::SetParam => "设置参数",
            Afn::QueryParam => "查询参数",
            Afn::DataForward => "数据转发",
            Afn::Control => "控制命令",
            Afn::StartNodeSearch => "启动从节点搜索",
            Afn::QueryFileTransferFailure => "查询文件传输失败节点",
            Afn::RouteQuery => "路由查询",
            Afn::FileTransfer => "文件传输",
            Afn::NotifyStoreNodeArchive => "通知存储从节点档案",
            Afn::Other(_) => "其他功能码",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_afn_roundtrip() {
        let afn = Afn::QueryParam;
        let byte = afn.to_byte();
        let afn2 = Afn::from_byte(byte);
        assert_eq!(afn, afn2);
    }

    #[test]
    fn test_afn_description() {
        assert_eq!(Afn::AckNack.description(), "确认/否认");
        assert_eq!(Afn::DataForward.description(), "数据转发");
    }

    #[test]
    fn test_afn_other() {
        let afn = Afn::from_byte(0xFF);
        assert_eq!(afn, Afn::Other(0xFF));
        assert_eq!(afn.to_byte(), 0xFF);
    }
}
