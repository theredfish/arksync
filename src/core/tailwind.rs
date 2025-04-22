use std::{collections::HashMap, sync::LazyLock};

// Static arrays of Tailwind classes pairs.
// This is used to overcome Tailwind limitations when
// parsing source files with viewports. This is preferred
// than a safelist that will increase the size of the output.
pub(crate) static MD_COL_SPAN_MAP: LazyLock<HashMap<i32, &'static str>> = LazyLock::new(|| {
    let mut map = HashMap::new();

    map.insert(1, "md:col-span-1");
    map.insert(2, "md:col-span-2");
    map.insert(3, "md:col-span-3");
    map.insert(4, "md:col-span-4");
    map.insert(5, "md:col-span-5");
    map.insert(6, "md:col-span-6");
    map.insert(7, "md:col-span-7");
    map.insert(8, "md:col-span-8");
    map.insert(9, "md:col-span-9");
    map.insert(10, "md:col-span-10");
    map.insert(11, "md:col-span-11");
    map.insert(12, "md:col-span-12");

    map
});

pub(crate) static MD_COL_START_MAP: LazyLock<HashMap<i32, &'static str>> = LazyLock::new(|| {
    let mut map = HashMap::new();

    map.insert(1, "md:col-start-1");
    map.insert(2, "md:col-start-2");
    map.insert(3, "md:col-start-3");
    map.insert(4, "md:col-start-4");
    map.insert(5, "md:col-start-5");
    map.insert(6, "md:col-start-6");
    map.insert(7, "md:col-start-7");
    map.insert(8, "md:col-start-8");
    map.insert(9, "md:col-start-9");
    map.insert(10, "md:col-start-10");
    map.insert(11, "md:col-start-11");
    map.insert(12, "md:col-start-12");

    map
});

pub(crate) static MD_ROW_START_MAP: LazyLock<HashMap<i32, &'static str>> = LazyLock::new(|| {
    let mut map = HashMap::new();

    map.insert(0, "md:row-start-auto");
    map.insert(1, "md:row-start-1");
    map.insert(2, "md:row-start-2");
    map.insert(3, "md:row-start-3");
    map.insert(4, "md:row-start-4");
    map.insert(5, "md:row-start-5");
    map.insert(6, "md:row-start-6");
    map.insert(7, "md:row-start-7");
    map.insert(8, "md:row-start-8");
    map.insert(9, "md:row-start-9");
    map.insert(10, "md:row-start-10");
    map.insert(11, "md:row-start-11");
    map.insert(12, "md:row-start-12");

    map
});

pub(crate) static MD_ROW_SPAN_MAP: LazyLock<HashMap<i32, &'static str>> = LazyLock::new(|| {
    let mut map = HashMap::new();

    map.insert(1, "md:row-span-1");
    map.insert(2, "md:row-span-2");
    map.insert(3, "md:row-span-3");
    map.insert(4, "md:row-span-4");
    map.insert(5, "md:row-span-5");
    map.insert(6, "md:row-span-6");
    map.insert(7, "md:row-span-7");
    map.insert(8, "md:row-span-8");
    map.insert(9, "md:row-span-9");
    map.insert(10, "md:row-span-10");
    map.insert(11, "md:row-span-11");
    map.insert(12, "md:row-span-12");

    map
});
