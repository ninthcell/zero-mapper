# ZeroMapper

**한국어** | [English](README.en.md) | [日本語](README.ja.md)

8BitDo Zero, 8BitDo Micro 같은 소형 컨트롤러를 **그림 작업용 단축키 패드**로 만들어주는 Windows 트레이 앱.

포커스된 앱을 자동으로 감지해서 앱마다 다른 버튼 매핑을 적용합니다. 클립스튜디오에서는 브러시/지우개/실행취소, 포토샵으로 전환하면 자동으로 포토샵 단축키로 바뀝니다.

---

## 왜 ZeroMapper인가

**CPU 점유율 0%에 수렴, 메모리 2MB.**

백그라운드 앱은 안 쓸 때도 자원을 먹으면 안 됩니다. ZeroMapper는 버튼을 안 누르면 폴링을 150ms로 늦추고, 화면 잠금이나 절전 상태에서는 완전히 멈춥니다. 항상 켜두어도 배터리나 성능에 영향이 없습니다.

- 설치 없음 — exe 하나와 config.toml 하나면 끝
- 앱 전환 시 프로필 자동 전환 — 창 제목 기반으로 즉시 감지
- **버튼 조합** — `LB+A`, `RB+DpadLeft` 같은 조합을 하나의 키보드 단축키에 매핑. 버튼이 적은 소형 컨트롤러에서도 수십 개의 단축키를 커버
- 탭/홀드 두 가지 출력 모드

---

## 설치

1. [Releases](../../releases)에서 최신 zip 파일 다운로드
2. 원하는 폴더에 압축 해제
3. `ZeroMapper.exe` 실행 → 시스템 트레이에 아이콘 등장

> **8BitDo 컨트롤러 XInput 모드 설정**: X 버튼을 누른 채로 START를 눌러 전원을 켜세요. 모드 스위치가 있는 모델은 **X** 위치로 맞추세요.

---

## 닌텐도 레이아웃 (A↔B, X↔Y 스왑)

8BitDo Zero, Micro 등 닌텐도 배열 컨트롤러는 XInput 모드에서 버튼 이름이 뒤바뀝니다.

```
        Xbox 배열              닌텐도 배열 (8BitDo)

         [Y]                       [X]
       [X] [B]                   [Y] [A]
         [A]                       [B]
```

기본 config는 **8BitDo Zero 2 기준 닌텐도 레이아웃(`nintendo_layout = true`)이 기본값**입니다. 컨트롤러에 **인쇄된 버튼 이름** 그대로 config에 적을 수 있습니다.

```toml
schema_version = 1
controller_player = 1
nintendo_layout = true   # 8BitDo Zero/Micro 등 닌텐도 배열 컨트롤러용
```

Xbox 컨트롤러를 사용하는 경우 `nintendo_layout = false`로 설정하거나 줄을 삭제하세요.

---

## 기본 매핑

설치 직후 Clip Studio Paint, Photoshop, Aseprite, Krita 프로필이 포함되어 있습니다.

### Clip Studio Paint

| 버튼 | 단축키 | 기능 |
|------|--------|------|
| A | P | 펜 |
| Y | B | 브러시 |
| B | E | 지우개 |
| X (홀드) | Space | 화면 이동 |
| DpadLeft | Ctrl+Z | 실행취소 |
| DpadRight | Ctrl+Y | 다시실행 |
| LB+Y | I | 색상 선택 도구 |
| LB+A | M | 선택 도구 |
| LB+B | K | 채우기 |
| RB+A | Ctrl+T | 변형 |
| Start | Ctrl+S | 저장 |

### Photoshop

| 버튼 | 단축키 | 기능 |
|------|--------|------|
| A | B | 브러시 |
| Y | I | 스포이드 |
| B | E | 지우개 |
| X (홀드) | Space | 화면 이동 |
| DpadLeft | Ctrl+Z | 실행취소 |
| DpadRight | Ctrl+Shift+Z | 다시실행 |
| LB+A | M | 선택 도구 |
| LB+B | V | 이동 도구 |
| LB+DpadLeft | [ | 브러시 크기 축소 |
| LB+DpadRight | ] | 브러시 크기 확대 |
| RB+DpadLeft | Ctrl+- | 축소 |
| RB+DpadRight | Ctrl+= | 확대 |
| RB+A | Ctrl+T | 자유 변형 |
| RB+B | X | 전경/배경색 전환 |
| Start | Ctrl+S | 저장 |

### Aseprite

| 버튼 | 단축키 | 기능 |
|------|--------|------|
| A | B | 브러시 |
| B | E | 지우개 |
| X (홀드) | Space | 화면 이동 |
| Y (홀드) | Alt | 스포이드 |
| LB+Y (홀드) | Ctrl | 다중 선택 |
| DpadLeft | Ctrl+Z | 실행취소 |
| DpadRight | Ctrl+Y | 다시실행 |
| LB+A | M | 선택 도구 |
| RB+X | Tab | UI 토글 |
| Start | Ctrl+S | 저장 |

### Krita

| 버튼 | 단축키 | 기능 |
|------|--------|------|
| A | B | 브러시 |
| B | E | 지우개 |
| X (홀드) | Space | 화면 이동 |
| Y (홀드) | Ctrl | 브러시 크기 조절 |
| DpadLeft | Ctrl+Z | 실행취소 |
| DpadRight | Ctrl+Shift+Z | 다시실행 |
| LB+A | Ctrl+R | 참조 이미지 |
| LB+B | V | 이동 도구 |
| LB+DpadLeft | [ | 브러시 크기 축소 |
| LB+DpadRight | ] | 브러시 크기 확대 |
| RB+B | X | 전경/배경색 전환 |
| RB+X | D | 기본 색상 |
| RB+Y | F5 | 브러시 설정 |
| Start | Ctrl+S | 저장 |

---

## 커스터마이징

트레이 메뉴 → "Open config.toml"로 설정 파일을 열고 수정한 뒤, 트레이 메뉴 → "Reload config"를 클릭하면 즉시 반영됩니다. 앱을 재시작할 필요 없습니다.

```toml
schema_version = 1
controller_player = 1  # 컨트롤러 플레이어 번호 (1~4)

[profiles.my_app]
name = "내 앱 이름"              # 트레이 메뉴에 표시될 이름
title_regex = "My App"          # 창 제목에 포함된 텍스트 (정규식 가능)

[profiles.my_app.map]
A = "P"                               # 탭: A 누르면 P 한 번 입력
X = { mode = "hold", send = "Space" } # 홀드: X 누르고 있는 동안 Space 유지
"LB+A" = "Ctrl+Z"                     # 콤보: LB와 A 동시에 누르면 Ctrl+Z
```

**콤보 우선 규칙**: `LB+A`와 `A`가 둘 다 매핑되어 있으면, LB+A를 누를 때 `LB+A` 매핑만 실행되고 `A`는 무시됩니다.

### 지원 버튼

`A` `B` `X` `Y` `LB` `RB` `LT` `RT` `Back` `Start` `L3` `R3` `DpadUp` `DpadDown` `DpadLeft` `DpadRight`

### 지원 키

`A`–`Z`, `0`–`9`, `F1`–`F24`, `Ctrl`, `Alt`, `Shift`, `Win`, `Enter`, `Esc`, `Space`, `Tab`, `Backspace`, `Delete`, `Insert`, `Home`, `End`, `PageUp`, `PageDown`, `Up`, `Down`, `Left`, `Right`, `CapsLock`, `[` `]` `\` `-` `=` `,` `.` `/` `;` `'`

---

## 빌드

```
cargo build --release
```

바이너리: `target\release\zero_mapper.exe`

---

## 제한 사항

컨트롤러 입력을 차단하지 않습니다. 키보드 입력을 추가로 전송하는 방식이므로, 원래의 XInput 신호는 그대로 다른 앱에도 전달됩니다.
