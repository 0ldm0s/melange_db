#!/usr/bin/env python3
import re
import sys

def remove_event_verifier_calls(file_path):
    with open(file_path, 'r', encoding='utf-8') as f:
        content = f.read()

    # 移除所有包含event_verifier的代码块
    # 匹配 #[cfg(feature = "for-internal-testing-only")] { ... } 块
    pattern = r'#\[cfg\(feature = "for-internal-testing-only"\)\]\s*\{[^}]*event_verifier[^}]*\}'

    # 使用非贪婪匹配
    content = re.sub(pattern, '', content, flags=re.DOTALL)

    with open(file_path, 'w', encoding='utf-8') as f:
        f.write(content)

    print(f"已处理文件: {file_path}")

if __name__ == "__main__":
    remove_event_verifier_calls("/Users/0ldm0s/workspaces/rust/melange_db/src/object_cache.rs")
    remove_event_verifier_calls("/Users/0ldm0s/workspaces/rust/melange_db/src/tree.rs")