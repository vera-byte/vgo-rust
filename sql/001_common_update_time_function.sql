-- ================================
-- 公共触发器函数迁移脚本
-- 创建通用的更新时间自动更新函数
-- ================================

-- ----------------------------
-- 公共触发器函数：自动更新update_time字段
-- 功能：在记录更新时自动将update_time字段设置为当前时间戳
-- 使用范围：所有包含update_time字段的表
-- ----------------------------
CREATE OR REPLACE FUNCTION public.update_update_time()
RETURNS TRIGGER AS $$
BEGIN
    -- 将update_time字段更新为当前时间戳
    NEW."update_time" = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 授予公共执行权限
GRANT EXECUTE ON FUNCTION public.update_update_time() TO PUBLIC;

-- 函数注释
COMMENT ON FUNCTION public.update_update_time() IS 
'公共触发器函数，用于自动更新update_time时间戳字段。
用法：在包含update_time字段的表上创建BEFORE UPDATE触发器。';

-- ----------------------------
-- 公共触发器函数：自动更新modified_at字段
-- 功能：在记录更新时自动将modified_at字段设置为当前时间戳
-- 使用范围：所有包含modified_at字段的表
-- ----------------------------
CREATE OR REPLACE FUNCTION public.update_modified_at()
RETURNS TRIGGER AS $$
BEGIN
    -- 将modified_at字段更新为当前时间戳
    NEW."modified_at" = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 授予公共执行权限
GRANT EXECUTE ON FUNCTION public.update_modified_at() TO PUBLIC;

-- 函数注释
COMMENT ON FUNCTION public.update_modified_at() IS 
'公共触发器函数，用于自动更新modified_at时间戳字段。
用法：在包含modified_at字段的表上创建BEFORE UPDATE触发器。';

-- ----------------------------
-- 工具函数：为现有表自动应用更新时间触发器
-- 功能：根据表名和时间字段名自动创建相应的触发器
-- ----------------------------
CREATE OR REPLACE FUNCTION public.apply_update_time_trigger(
    table_name TEXT, 
    time_field TEXT DEFAULT 'update_time'
)
RETURNS VOID AS $$
DECLARE
    function_name TEXT;
BEGIN
    -- 根据时间字段名确定使用哪个函数
    IF time_field = 'update_time' THEN
        function_name := 'update_update_time';
    ELSIF time_field = 'modified_at' THEN
        function_name := 'update_modified_at';
    ELSE
        RAISE EXCEPTION '不支持的时间字段: %', time_field;
    END IF;
    
    -- 删除已存在的触发器并创建新触发器
    EXECUTE format('
        DROP TRIGGER IF EXISTS trigger_%1$s_update_time ON %2$I;
        CREATE TRIGGER trigger_%1$s_update_time
            BEFORE UPDATE ON %2$I
            FOR EACH ROW
            EXECUTE FUNCTION public.%3$I();
    ', table_name, table_name, function_name);
END;
$$ LANGUAGE plpgsql;

-- 函数注释
COMMENT ON FUNCTION public.apply_update_time_trigger(TEXT, TEXT) IS 
'工具函数：为任意表自动应用更新时间触发器。
参数：table_name-表名，time_field-时间字段名(默认update_time)';

-- ================================
-- 公共函数创建完成
-- ================================