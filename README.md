# Entscript

**Entscript**는 텍스트로 작성한 코드를 Entry 블록 프로젝트 데이터로 변환하는 프로그래밍 언어 및 컴파일러입니다.

```text
Entscript source (.ents)
        |
        v
 Lexer -> Parser -> Analyzer
        |
        v
 Entry project JSON
        |
        v
 gzip TAR package (.ent)
```

이 문서는 현재 저장소의 구현을 기준으로 작성한 공식 사용 설명서입니다.

## 목차

- [시작하기](#시작하기)
- [설치 및 빌드](#설치-및-빌드)
- [CLI 사용법](#cli-사용법)
- [프로젝트 설정](#프로젝트-설정-settingsjson)
- [에셋 디렉터리](#에셋-디렉터리)
- [언어 문법](#언어-문법)
- [변수와 리스트](#변수와-리스트)
- [함수와 매개변수](#함수와-매개변수)
- [Entry 블록 사용](#entry-블록-사용)
- [블록 지원 범위](#블록-지원-범위)
- [Entry 프로젝트 가져오기](#entry-프로젝트-가져오기)
- [Rust API](#rust-api)
- [문제 해결](#문제-해결)
- [현재 제한 사항](#현재-제한-사항)

---

## 시작하기

### 요구 사항

- Rust toolchain (`cargo`, `rustc`)
- `tar` 명령어가 `PATH`에 등록되어 있어야 합니다.
- Entry에서 생성하거나 가져올 프로젝트 파일을 사용하려면 해당 프로젝트의 JSON 데이터가 필요합니다.

Entscript CLI는 컴파일된 Entry 프로젝트 JSON과 프로젝트가 참조하는 이미지·썸네일·사운드 에셋을 gzip 압축된 TAR 아카이브로 패키징합니다. `.ent` 내부에는 `temp/project.json`과 `temp/` 기준의 에셋 경로가 저장됩니다.

### 첫 번째 프로그램

`hello.ents` 파일을 만듭니다.

```entscript
@엔트리봇 {
    when start {
        say("Hello, Entry!")
        wait(1)
        say("Entscript is working.")
    }
}
```

프로젝트 루트에서 다음 명령을 실행합니다.

```bash
cargo run -- hello.ents
```

성공하면 `hello.ent`가 생성됩니다. 이 파일을 Entry에서 가져와 실행할 수 있습니다.

---

## 설치 및 빌드

저장소를 내려받은 뒤 프로젝트 루트에서 실행합니다.

```bash
cargo build --release
```

실행 파일은 다음 위치에 생성됩니다.

```text
target/release/entscript
```

Cargo의 바이너리 설치 경로에 설치하려면 다음을 실행합니다.

```bash
cargo install --path .
```

설치 후에는 어느 디렉터리에서든 `entscript` 명령을 사용할 수 있습니다.

개발 중에는 다음 명령으로 테스트를 실행할 수 있습니다.

```bash
cargo test
```

---

## CLI 사용법

Entscript CLI는 두 가지 컴파일 모드를 제공합니다.

1. **프로젝트 빌드 모드**: `settings.json`을 읽고 프로젝트의 진입 파일을 컴파일합니다.
2. **직접 컴파일 모드**: 지정한 `.ents` 파일 하나를 바로 컴파일합니다.

### 프로젝트 빌드 모드

```bash
entscript build [OPTIONS]
```

| 옵션 | 설명 |
|---|---|
| `-C`, `--project-dir <DIR>` | 프로젝트 디렉터리. 기본값은 현재 디렉터리입니다. |
| `--settings <FILE>` | 설정 파일 경로. 기본값은 `settings.json`입니다. 프로젝트 디렉터리 기준 상대 경로입니다. |
| `-i`, `--entry <FILE>` | `build.entry`를 덮어쓸 진입 소스 파일입니다. |
| `-o`, `--output <FILE>` | `build.output`을 덮어쓸 출력 `.ent` 경로입니다. |
| `--name <NAME>` | `metadata.name`을 덮어씁니다. |
| `-p`, `--project <FILE>` | 기존 Entry 프로젝트 JSON을 불러와 해당 프로젝트를 기반으로 빌드합니다. |

예시:

```bash
# 현재 디렉터리의 settings.json으로 빌드
entscript build

# 작품 이름 덮어쓰기
entscript build --name "My Game"

# 출력 파일 경로 덮어쓰기
entscript build --output dist/game.ent

# 다른 프로젝트 디렉터리 빌드
entscript build --project-dir ./project

# 진입 파일과 출력 경로를 한 번에 지정
entscript build --entry src/main.ents --output dist/game.ent

# 기존 Entry 프로젝트 데이터를 보존하면서 스크립트 갱신
entscript build --project existing-project.json
```

`cargo run`으로 개발 중 실행할 때는 다음처럼 입력합니다.

```bash
cargo run -- build
cargo run -- build --project-dir examples/project
```

### 직접 컴파일 모드

```bash
entscript <SOURCE> [OPTIONS]
```

| 옵션 | 설명 |
|---|---|
| `<SOURCE>` | 컴파일할 Entscript 파일입니다. `-`이면 표준 입력을 읽습니다. |
| `-o`, `--output <FILE>` | 출력 `.ent` 경로입니다. 생략하면 입력 파일의 확장자를 `.ent`로 바꿉니다. |
| `-p`, `--project <FILE>` | 기존 Entry 프로젝트 JSON을 기반으로 컴파일합니다. |
| `-h`, `--help` | 도움말을 출력합니다. |
| `-V`, `--version` | 버전을 출력합니다. |

예시:

```bash
# hello.ents -> hello.ent
entscript hello.ents

# 출력 경로 지정
entscript hello.ents -o output/game.ent

# 기존 Entry 프로젝트에 스크립트를 반영
entscript hello.ents --project project.json -o updated.ent

# 표준 입력으로 컴파일하고 아카이브를 표준 출력으로 내보내기
cat hello.ents | entscript - -o - > hello.ent

# 도움말 및 버전
entscript --help
entscript --version
```

### 출력 패키지 구조

컴파일 결과 `.ent`는 gzip으로 압축된 TAR 아카이브입니다.

```text
hello.ent
└── temp/
    ├── project.json
    ├── ab/
    │   └── cd/
    │       ├── image/<file-id>.png
    │       ├── thumb/<file-id>.png
    │       └── sound/<file-id>.mp3
    └── ef/
        └── gh/
            └── sound/<file-id>.mp3
```

아카이브 내부를 확인하려면 다음 명령을 사용합니다.

```bash
tar -tzf hello.ent
```

CLI는 시스템의 gzip 지원 `tar` 프로그램을 실행합니다. 따라서 gzip 압축을 지원하는 `tar`가 설치되어 있고 `PATH`에서 검색 가능해야 합니다.

---

## 프로젝트 설정 (`settings.json`)

프로젝트 빌드 모드에서는 프로젝트 루트의 `settings.json`을 읽습니다.

### 권장 프로젝트 구조

```text
project/
├── settings.json
├── main.ents
├── modules/
│   └── player.ents
├── assets/
│   ├── ab/
│   │   └── cd/
│   │       ├── image/
│   │       │   └── abcd1234.png
│   │       └── thumb/
│   │           └── abcd1234.png
│   └── ef/
│       └── gh/
│           └── sound/
│               └── efgh5678.mp3
└── build/
```

현재 CLI는 `build.entry`에 지정된 **단일 진입 파일**을 읽습니다. `modules/`는 프로젝트 파일을 정리하기 위한 예시 디렉터리이며, 현재 빌드 명령이 해당 폴더의 소스를 자동으로 찾아 합치지는 않습니다.

### 에셋 디렉터리

`assets/`는 Entry 프로젝트 JSON이 참조하는 실제 이미지·썸네일·사운드 파일을 보관하는 디렉터리입니다. Entry는 파일 ID의 앞 4글자를 2글자씩 나눠 폴더를 구성합니다.

```text
assets/
└── ab/
    └── cd/
        ├── image/abcd1234.png
        ├── thumb/abcd1234.png
        └── sound/abcd1234.mp3
```

예를 들어 파일 ID가 `abcd1234`라면 저장 경로는 `assets/ab/cd/image/abcd1234.png`와 같은 형태입니다. `.ent` 안에서는 `temp/ab/cd/image/abcd1234.png`로 들어갑니다. 확장자와 실제 파일명은 프로젝트 JSON의 에셋 경로와 일치해야 합니다.

빌드 시 다음 위치의 `assets/`를 자동으로 읽습니다.

- `entscript build`: `--project-dir`로 지정한 프로젝트 디렉터리의 `assets/`
- `entscript main.ents`: 명령을 실행한 현재 디렉터리의 `assets/`

Entry가 프로젝트 JSON의 `fileurl` 또는 `thumbUrl`에서 이 해시 경로 형식을 참조하면, 빌더는 대응 파일이 실제로 존재하는지 검사합니다. 파일이 빠져 있으면 불완전한 `.ent`를 만들지 않고 오류를 반환합니다. **프로젝트 JSON이 실제로 참조하는 파일만** 패키지에 복사합니다. 참조되지 않는 에셋이나 `assets/thumbnail.png`처럼 Entry 에셋 경로 규칙에 맞지 않는 일반 파일은 복사하지 않습니다. 심볼릭 링크가 참조 경로에 포함되면 오류를 반환하며, 다른 위치의 심볼릭 링크는 에셋으로 따라가지 않습니다.

`metadata.thumbnail`은 위 에셋 디렉터리의 파일을 자동 등록하는 기능이 아닙니다. 해당 설정은 별도의 메타데이터 문자열로 저장되며, 실제 오브젝트 이미지나 썸네일을 사용하려면 Entry 프로젝트 JSON의 에셋 참조와 파일 경로가 함께 있어야 합니다.

### 설정 파일 예시

```json
{
  "version": 1,
  "metadata": {
    "name": "Hello World",
    "description": "Entscript로 만든 첫 작품",
    "category": "game",
    "tags": [
      "Entscript",
      "Example"
    ],
    "thumbnail": "assets/thumbnail.png"
  },
  "build": {
    "entry": "main.ents",
    "output": "build/hello.ent"
  }
}
```

### 최상위 속성

| 속성 | 타입 | 필수 | 설명 |
|---|---|---:|---|
| `version` | integer | 예 | 설정 파일 규격 버전입니다. 현재 `1`만 지원합니다. |
| `metadata` | object | 아니요 | 작품 메타데이터입니다. |
| `build` | object | 아니요 | 빌드 입력 및 출력 설정입니다. |

### `metadata`

| 속성 | 타입 | 설명 |
|---|---|---|
| `name` | string | Entry 프로젝트 최상위 `name` 필드에 적용됩니다. |
| `description` | string | 프로젝트 JSON의 최상위 `description` 필드에 저장됩니다. |
| `category` | string | 프로젝트 JSON의 최상위 `category` 필드에 저장됩니다. |
| `tags` | string array | 프로젝트 JSON의 최상위 `tags` 배열에 저장됩니다. |
| `thumbnail` | string 또는 `null` | 썸네일 경로 문자열을 저장합니다. `null`은 기존 값을 명시적으로 지웁니다. |

`name`은 컴파일러가 생성하는 Entry 프로젝트의 이름을 덮어씁니다. `description`, `category`, `tags`, `thumbnail`은 현재 컴파일러가 프로젝트 JSON 최상위에 추가 메타데이터로 기록하는 필드입니다. Entry 편집기나 게시 화면에서 이 추가 필드를 표시하거나 사용한다는 보장은 없습니다.

`thumbnail`은 입력한 경로 문자열만 그대로 기록합니다. 경로를 프로젝트 디렉터리 기준으로 해석하거나 이미지 파일을 `.ent`에 포함하지 않으며, Entry 서버에도 업로드하지 않습니다.

### `build`

| 속성 | 타입 | 기본값 | 설명 |
|---|---|---|---|
| `entry` | string | `main.ents` | 컴파일할 진입 Entscript 파일입니다. |
| `output` | string | 진입 파일 이름의 확장자를 `.ent`로 변경한 경로 | 생성할 패키지 경로입니다. |

### 경로 기준

`--project-dir`로 지정한 프로젝트 디렉터리가 기준입니다.

- `build.entry`
- `build.output`
- `--settings`
- `--project`

위 경로가 상대 경로이면 프로젝트 디렉터리를 기준으로 해석합니다. 절대 경로는 그대로 사용합니다.

### 설정 우선순위

설정 값은 다음 순서로 결정됩니다.

1. CLI 옵션
2. `settings.json`
3. 컴파일러 기본값

예를 들어 `settings.json`에 `"name": "Hello"`가 있어도 다음 명령을 실행하면 출력 프로젝트 이름은 `My Game`이 됩니다.

```bash
entscript build --name "My Game"
```

### 설정 파일 유효성 검사

- `settings.json`이 없으면 경고를 출력하고 기본 설정을 사용합니다.
- `version`은 필수이며 정수 `1`이어야 합니다.
- 잘못된 JSON, 잘못된 자료형, 지원하지 않는 버전은 빌드 오류입니다.
- 알 수 없는 속성은 경고를 출력하고 무시합니다.
- 생략된 메타데이터는 컴파일러 또는 기존 프로젝트의 값을 유지합니다.
- `metadata.thumbnail`에 `null`을 지정하면 기존 썸네일 값을 지웁니다.

`settings.json`이 없는 경우 기본 진입 파일은 `main.ents`입니다.

---

## 언어 문법

### 기본 구조

Entscript 프로그램은 최상위 선언과 오브젝트 섹션으로 구성됩니다.

```entscript
var score = 0

fn announce(message) {
    say(message)
}

@엔트리봇 {
    when start {
        announce("시작!")
    }
}
```

최상위 선언 종류:

- `var`: 전역 변수 선언
- `list`: 전역 리스트 선언
- `fn`: 사용자 정의 함수 선언
- `@오브젝트이름 { ... }`: Entry 오브젝트의 이벤트 스크립트 선언

오브젝트 이름은 Entry 프로젝트에 실제로 존재해야 합니다. 기본 프로젝트 템플릿에는 `엔트리봇` 오브젝트가 포함되어 있습니다. 다른 오브젝트를 사용하려면 기존 Entry 프로젝트 JSON을 `--project`로 전달해야 합니다.

### 주석

한 줄 주석은 `//`로 시작합니다.

```entscript
// 점수를 초기화합니다.
var score = 0
```

### 식별자

식별자는 밑줄 또는 문자로 시작할 수 있으며, 이후에는 문자·숫자·밑줄을 사용할 수 있습니다. 한글 식별자도 사용할 수 있습니다.

```entscript
var player_score = 0
var 플레이어이름 = "Ada"
```

### 리터럴

```entscript
42
3.14
"Hello"
true
false
null
```

문자열은 큰따옴표를 사용합니다. 지원하는 이스케이프는 다음과 같습니다.

| 표기 | 의미 |
|---|---|
| `\n` | 줄바꿈 |
| `\r` | 캐리지 리턴 |
| `\t` | 탭 |
| `\0` | NUL 문자 |
| `\"` | 큰따옴표 |
| `\\` | 역슬래시 |

문자열 안에 실제 줄바꿈을 직접 넣을 수는 없습니다.

### 표현식

지원하는 연산자:

| 연산자 | 의미 |
|---|---|
| `!value` | 논리 부정 |
| `-value` | 부호 반전 |
| `*`, `/`, `%` | 곱셈, 나눗셈, 나머지 |
| `+`, `-` | 덧셈, 뺄셈 |
| `<`, `<=`, `>`, `>=` | 크기 비교 |
| `==`, `!=` | 동등 비교 |
| `&&` | 논리 AND |
| `\|\|` | 논리 OR |

우선순위는 일반적인 산술·비교·논리 연산 순서를 따릅니다. 괄호로 계산 순서를 명시할 수 있습니다.

```entscript
var result = 10 + 20 * 3

@엔트리봇 {
    when start {
        say((10 + 20) * 3)
        if score >= 10 && score < 100 {
            say("레벨 업!")
        }
    }
}
```

### 블록 호출

일반적인 블록 호출은 함수 호출과 같은 형태입니다.

```entscript
say("Hello")
wait(1)
move(10)
```

인자가 없는 블록은 빈 괄호를 사용합니다.

```entscript
show()
hide()
```

블록에 연결된 내부 문장 영역은 중괄호로 작성합니다.

```entscript
loop {
    move(10)
    wait(0.2)
}

if score > 10 {
    say("높음")
} else {
    say("낮음")
}
```

`else`는 `if` 블록에 연결되는 두 번째 문장 영역입니다.

---

## 변수와 리스트

### 전역 변수

`var`로 전역 변수를 선언합니다.

```entscript
var score = 0
var player_name = "Ada"
var is_ready = true
```

초기값은 문자열, 숫자, 불리언, `null`과 같은 리터럴이어야 합니다. 변수 초기값에 계산식이나 다른 변수 참조를 사용할 수 없습니다.

변수 관련 블록:

```entscript
var score = 0

@엔트리봇 {
    when start {
        set(score, 10)
        change(score, 5)
        say(score)

        show_variable(score)
        hide_variable(score)
    }
}
```

- `set(variable, value)`: 변수 값을 설정합니다.
- `change(variable, value)`: 변수 값에 수를 더합니다.
- `show_variable(variable)`: 변수 표시를 켭니다.
- `hide_variable(variable)`: 변수 표시를 끕니다.
- 변수 이름은 식별자로 전달합니다.

기존 Entry 프로젝트의 전역 변수와 오브젝트 지역 변수도 이름으로 참조할 수 있습니다. `@오브젝트이름` 안에서는 해당 오브젝트의 지역 변수를 먼저 찾고, 없으면 전역 변수를 찾습니다. 함수 본문은 전역 변수 범위에서 분석됩니다. 새 `var` 선언은 기존 프로젝트의 같은 이름 변수와 충돌하면 오류가 발생합니다.

### 리스트

`list`로 전역 리스트를 선언합니다.

```entscript
list inventory = ["coin", "key"]
list empty_items = []
```

초기 항목은 리터럴이어야 합니다.

리스트 조작:

```entscript
@엔트리봇 {
    when start {
        list_add(inventory, "gem")
        list_insert(inventory, 2, "map")
        list_set(inventory, 1, "ticket")
        list_remove(inventory, 2)

        say(list_get(inventory, 1))
        say(list_length(inventory))

        if list_contains(inventory, "gem") {
            say("보석을 가지고 있습니다.")
        }

        list_show(inventory)
        list_hide(inventory)
    }
}
```

| 함수 | 설명 |
|---|---|
| `list_add(list, value)` | 리스트 끝에 항목을 추가합니다. |
| `list_insert(list, index, value)` | 지정한 위치에 항목을 삽입합니다. |
| `list_set(list, index, value)` | 지정한 위치의 항목을 변경합니다. |
| `list_remove(list, index)` | 지정한 위치의 항목을 삭제합니다. |
| `list_get(list, index)` | 지정한 위치의 항목을 가져옵니다. |
| `list_length(list)` | 항목 개수를 가져옵니다. |
| `list_contains(list, value)` | 값이 리스트에 있는지 판단합니다. |
| `list_show(list)` | 리스트 표시를 켭니다. |
| `list_hide(list)` | 리스트 표시를 끕니다. |

리스트 인덱스는 Entry와 동일하게 **1부터 시작**합니다.

기존 Entry 프로젝트에 있는 리스트도 이름으로 참조할 수 있습니다. `@오브젝트이름` 안에서는 해당 오브젝트의 지역 리스트를 먼저 찾고, 없으면 전역 리스트를 찾습니다. 리스트 이름은 식별자나 문자열로 지정할 수 있습니다. 다만 Entscript의 `var`와 `list` 선언은 현재 전역 범위만 지원합니다.

---

## 함수와 매개변수

### 문장 함수

`fn`으로 함수를 선언합니다. 반환 표현식이 없는 함수는 문장 함수이며, 블록 문장처럼 호출합니다.

```entscript
fn announce(message) {
    say(message)
}

@엔트리봇 {
    when start {
        announce("Hello!")
    }
}
```

### 값 함수

`->` 뒤에 반환 표현식을 작성하면 값 함수가 됩니다. 값 함수는 다른 블록의 입력이나 표현식 안에서 사용할 수 있습니다.

```entscript
fn add(a, b) -> a + b {
}

@엔트리봇 {
    when start {
        say(add(10, 20))
    }
}
```

현재 문법은 `return` 문을 사용하지 않습니다. 반환값은 함수 선언의 `->` 뒤에 표현식으로 지정합니다.

### 매개변수 타입

매개변수는 기본적으로 값 매개변수입니다.

```entscript
fn show_message(message) {
    say(message)
}
```

불리언 매개변수는 `: bool` 또는 `: boolean`으로 지정합니다. 값 매개변수는 `: value` 또는 `: string`으로 명시할 수 있습니다.

```entscript
fn announce(message: string, condition: bool) {
    if condition {
        say(message)
    }
}
```

함수 호출 시 인자 개수를 검사합니다. 컴파일러가 인자 타입을 판별할 수 있는 경우 값/불리언 타입 불일치도 오류로 처리합니다. 기존 Entry 프로젝트의 사용자 함수도 `func_<함수ID>(...)` 형태로 직접 호출할 수 있습니다. 이 경우 저장된 함수 블록의 매개변수 리본을 읽어 인자 개수와 확인 가능한 값/불리언 타입을 검사합니다.

### 함수 호출 규칙

- 함수는 최상위에서 선언합니다.
- 함수 이름은 내장 블록 이름과 충돌할 수 없습니다.
- 함수 이름과 매개변수 이름은 중복될 수 없습니다.
- 함수는 다른 함수보다 앞이나 뒤에 선언되어도 호출할 수 있습니다.
- 함수 간 전방 참조와 재귀 호출을 위한 ID를 미리 할당합니다.
- 문장 함수는 문장 위치에서 호출합니다.
- 값 함수는 표현식 위치에서 호출합니다.

---

## Entry 블록 사용

Entscript의 기본 블록 별칭은 Entry의 네이티브 블록 타입으로 변환됩니다. 블록 호출은 인자와 내부 문장 영역을 기준으로 작성합니다.

### 이벤트

```entscript
@엔트리봇 {
    when start {
        say("시작 버튼")
    }

    when key("81") {
        say("Q 키 입력")
    }

    when mouse_click {
        say("마우스 클릭")
    }

    when scene_start {
        say("장면 시작")
    }
}
```

지원되는 이벤트 별칭에는 다음이 포함됩니다.

- `start`
- `key`, `key_pressed`
- `mouse_click`, `mouse_release`
- `object_click`, `object_release`
- `message`
- `scene_start`
- `clone_start`

일부 이벤트는 Entry 프로젝트의 신호·장면·키 설정과 연결됩니다. 이벤트 인자는 해당 Entry 블록 정의가 요구하는 값으로 전달해야 합니다.

### 블록 별칭 카테고리

현재 Registry는 일반적인 Entry 블록에 대해 친숙한 별칭을 제공합니다.

| 카테고리 | 주요 지원 기능 |
|---|---|
| 시작 | 시작 버튼, 키·마우스·오브젝트 이벤트, 신호, 장면, 복제본 이벤트 |
| 흐름 | 대기, 반복, 조건, 반복 중단/계속, 스크립트 중단, 장면 및 복제본 제어 |
| 움직임 | 이동, 좌표 변경/설정, 마우스·오브젝트 위치, 방향, 회전, 시간 기반 이동 |
| 생김새 | 말하기/생각하기, 보이기/숨기기, 모양 변경, 효과, 크기, 뒤집기, 레이어 |
| 붓 | 도장, 그리기 시작/중지, 색상, 채우기, 굵기, 투명도 |
| 글상자 | 글쓰기, 앞/뒤에 추가, 글꼴, 글자·배경 색상, 내용 읽기 |
| 소리 | 소리 재생, 구간 재생, 완료 대기, 볼륨, 정지, 배경음악, 재생 속도 |
| 판단 | 비교, 논리 연산, 클릭·키 입력·충돌 판단, 오브젝트 타입 검사 |
| 계산 | 산술, 난수, 수학 함수, 문자열, 날짜/시간, 좌표/거리, 타이머, RGB/HEX 변환 |
| 자료 | 변수, 리스트, 대답 및 표시 설정 |
| 함수 | 문장 함수, 값 함수, 값/불리언 매개변수 |
| 데이터 분석 | Entry 데이터 테이블을 대상으로 하는 행·열·셀·통계·상관관계 블록 |

이 표는 카테고리별 기능 요약입니다. 모든 위키 블록에 전용 Entscript 별칭이 있다는 뜻은 아닙니다. 별칭이 없는 일반 네이티브 블록은 아래 `raw_block()`을 사용합니다.

### 블록 선택값

블록의 드롭다운 또는 필드 인자는 일반 문자열과 구분해야 합니다. `field()`는 Entry 필드 값을 직접 전달합니다.

```entscript
@엔트리봇 {
    when start {
        change_layer(field("front"))
        set_effect(field("color"), 50)
    }
}
```

선택값은 실제 Entry 블록 정의가 요구하는 문자열이어야 합니다. 블록마다 사용하는 값이 다르므로 임의의 문자열을 넣으면 Entry에서 기대한 동작을 하지 않을 수 있습니다.

### 네이티브 블록 직접 사용: `raw_block()`

전용 별칭이 없는 일반 Entry 블록은 `raw_block()`으로 직접 지정할 수 있습니다.

```entscript
@엔트리봇 {
    when start {
        raw_block("some_native_type", field("DROPDOWN_VALUE"), raw(12), null)
    }
}
```

`raw_block()`의 첫 번째 인자는 Entry 네이티브 블록의 정확한 타입 ID 문자열입니다. 이후 인자는 해당 블록의 `def.params` 순서와 일치해야 합니다.

| 표현식 | 의미 |
|---|---|
| `field("VALUE")` | Entry 필드/드롭다운 문자열을 그대로 전달합니다. |
| `raw(12)` | 숫자 리터럴을 중첩 리포터 블록이 아닌 원시 값으로 전달합니다. |
| `raw(true)` | 불리언 원시 값을 전달합니다. |
| `raw("text")` | 문자열 원시 값을 전달합니다. |
| `raw(null)` | 원시 `null` 값을 전달합니다. |
| `null` | 비어 있는 Entry 파라미터 슬롯입니다. |
| `"text"` | 일반 문자열 값 리포터 블록으로 변환됩니다. |
| `12` | 일반 숫자 값 리포터 블록으로 변환됩니다. |
| `reporter(...)` | 다른 블록의 파라미터 안에 중첩되는 리포터 블록입니다. |

문장 영역이 하나인 블록은 `{ ... }`를 사용합니다. 두 개의 문장 영역이 있는 블록은 `else`로 두 번째 영역을 지정합니다.

```entscript
raw_block("native_two_branch_type", raw(true), null) {
    say("첫 번째 영역")
} else {
    say("두 번째 영역")
}
```

네이티브 이벤트 ID도 `when` 뒤에 직접 사용할 수 있습니다.

```entscript
@엔트리봇 {
    when when_some_key_pressed(field("81")) {
        say("키 입력")
    }
}
```

`entry_block()`도 `raw_block()`과 같은 용도의 별칭입니다.

**주의:** `raw_block()`은 블록 타입과 파라미터를 그대로 구성하는 저수준 기능입니다. 정확한 타입 ID, 파라미터 순서, 필드 값, 문장 영역 개수는 Entry 블록 정의를 확인한 뒤 사용해야 합니다. 확장·AI 블록은 외부 모듈이나 모델 설정이 필요하므로 ID만 전달한다고 동작하지 않습니다.

### 데이터 분석 테이블

데이터 분석 별칭은 기존 Entry 프로젝트에 등록된 테이블 ID를 사용합니다.

```entscript
@엔트리봇 {
    when start {
        append_row_to_table("TABLE_ID", "ROW")
        insert_row_to_table("TABLE_ID", 2, "ROW")
        set_value_from_table("TABLE_ID", 2, "점수", 100)

        say(table_count("TABLE_ID", "ROW"))
        say(table_value("TABLE_ID", 2, "점수"))
        say(table_stat("TABLE_ID", "점수", "AVG"))
        say(table_correlation("TABLE_ID", "점수", "시간"))

        open_table("TABLE_ID")
        open_table_chart("TABLE_ID", "0")
    }
}
```

`TABLE_ID`는 실제 프로젝트의 `tables` 메타데이터에 존재하는 ID로 바꿔야 합니다. 새 프로젝트를 기본 템플릿으로 생성한 경우 테이블이 없으므로, 테이블이 포함된 Entry 프로젝트 JSON을 `--project`로 전달해야 합니다.

---

## 블록 지원 범위

Entry 위키의 블록 카테고리는 시작, 흐름, 움직임, 생김새, 글상자, 붓, 소리, 판단, 계산, 자료, 함수, 데이터 분석, 인공지능, 확장, 하드웨어로 구분됩니다.

### 일반 블록

일반적인 내장 블록 카테고리는 다수의 전용 별칭으로 지원합니다. 일부 덜 사용되는 블록이나 새로 추가된 블록은 아직 별칭이 없을 수 있습니다. 이 경우 정확한 블록 정의를 알고 있다면 `raw_block()`으로 사용할 수 있습니다.

### 아직 구현되지 않은 영역

| 영역 | 현재 상태 |
|---|---|
| 인공지능 | 번역, 읽어주기, 음성 인식, 사람/얼굴/사물/손 인식, 이미지·텍스트·소리·숫자 분류, 숫자 예측·군집 등은 구현하지 않았습니다. |
| 확장 | 날씨, 재난문자, 행사 정보 등 외부 확장 모듈은 구현하지 않았습니다. |
| 하드웨어 | 보드·로봇 연결, 펌웨어 설정, 장치별 런타임 연동은 관리하지 않습니다. |

AI, 확장, 하드웨어 블록은 단순히 Entry 블록 ID를 생성하는 것만으로 완전히 동작하지 않습니다. 모델 학습·프로젝트 설정·외부 모듈·장치 런타임 등의 추가 구성이 필요합니다.

Entry 위키에 설명된 테이블 가져오기, AI 모델 학습 등 편집기 팝업 작업은 실행 블록이 아니므로 Entscript 컴파일 대상이 아닙니다.

---

## Entry 프로젝트 가져오기

기존 Entry 프로젝트의 오브젝트, 변수, 리스트, 함수, 장면, 테이블 등의 정보를 보존하면서 Entscript 스크립트를 반영하려면 기존 프로젝트 JSON을 전달합니다.

```bash
entscript build --project existing-project.json
```

또는 직접 컴파일 모드에서:

```bash
entscript main.ents --project existing-project.json -o updated.ent
```

Entscript의 `@오브젝트이름`은 전달한 프로젝트의 실제 오브젝트 이름과 일치해야 합니다. 이름이 없거나 중복되면 컴파일 오류가 발생합니다.

기존 프로젝트 JSON의 스크립트는 Entscript에서 선언한 해당 오브젝트의 스크립트로 교체됩니다. 프로젝트 변수·리스트·함수 메타데이터를 검사하고, 오브젝트 지역 변수와 전역 변수의 범위를 구분해 참조합니다. 프로젝트의 나머지 데이터는 컴파일러가 수정하는 대상이 아닌 한 유지됩니다.

---

## Rust API

Entscript 컴파일러는 라이브러리로도 사용할 수 있습니다.

### 기본 프로젝트 생성

```rust
use entscript_converter::compile_to_json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
        @엔트리봇 {
            when start {
                say("Hello from Rust!")
            }
        }
    "#;

    let project_json = compile_to_json(source)?;
    std::fs::write("project.json", project_json)?;

    Ok(())
}
```

### 기존 프로젝트 기반 컴파일

```rust
use entscript_converter::compile_with_project_json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = r#"
        @엔트리봇 {
            when start {
                say("Updated!")
            }
        }
    "#;

    let existing_project = std::fs::read_to_string("project.json")?;
    let output_json = compile_with_project_json(source, &existing_project)?;

    std::fs::write("updated-project.json", output_json)?;

    Ok(())
}
```

공개 API:

| 함수 | 설명 |
|---|---|
| `compile_to_json(source)` | 기본 Entry 프로젝트 템플릿을 사용해 컴파일합니다. |
| `compile_with_project_json(source, project_json)` | 전달된 Entry 프로젝트 JSON을 기반으로 컴파일합니다. |
| `compile_with_project_value(source, project)` | 이미 파싱된 `serde_json::Value`를 기반으로 컴파일합니다. |

### WebAssembly

`wasm` feature를 활성화하고 `wasm32-unknown-unknown` 타깃으로 빌드하면 WebAssembly API를 사용할 수 있습니다.

```bash
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown --features wasm
```

WASM 공개 함수는 `compile(source)`와 `compile_with_project(source, project_json)`입니다.

---

## 문제 해결

### `a bin target must be available for cargo run`

실행 파일 타깃이 없는 프로젝트에서 발생하는 오류입니다. Entscript 저장소에는 `src/main.rs`를 사용하는 `entscript` 바이너리 타깃이 포함되어 있어야 합니다. 저장소 루트에서 다음 명령을 실행합니다.

```bash
cargo run -- --help
```

### `tar` 명령을 찾을 수 없음

`.ent` 패키지는 시스템 `tar` 명령으로 생성합니다. Linux/WSL 환경에서는 `tar`가 설치되어 있는지 확인하고, `PATH`에서 실행 가능한지 확인합니다.

```bash
tar --version
```

### 오브젝트를 찾을 수 없음

```text
Object '@엔트리봇' does not exist in the Entry project
```

소스의 `@오브젝트이름`과 기존 Entry 프로젝트의 오브젝트 이름이 일치하는지 확인합니다. 다른 프로젝트를 대상으로 빌드할 경우 `--project` 옵션을 사용합니다.

### 알 수 없는 블록

```text
Unknown block 'some_name'
```

블록 별칭의 철자를 확인합니다. 별칭이 등록되어 있지 않은 일반 Entry 블록이라면 정확한 Entry 타입 ID와 파라미터 구조를 확인한 뒤 `raw_block()`을 사용합니다.

### 설정 파일 오류

- JSON 문법이 올바른지 확인합니다.
- `version`이 정수 `1`인지 확인합니다.
- `metadata.tags`가 문자열 배열인지 확인합니다.
- `metadata.thumbnail`이 문자열 또는 `null`인지 확인합니다.
- `build.entry`, `build.output`이 비어 있지 않은 문자열인지 확인합니다.

---

## 현재 제한 사항

현재 구현에서 지원하지 않는 기능은 다음과 같습니다.

- 함수 오버로딩
- 기본 매개변수
- 함수 지역 변수
- 참조/포인터 매개변수
- 전역 변수 및 리스트 외의 지역 범위 선언
- 프로젝트 디렉터리 내 여러 `.ents` 파일의 자동 탐색·병합
- `metadata.thumbnail`에 지정한 별도 메타데이터 썸네일의 자동 등록 및 서버 업로드
- AI·확장·하드웨어 런타임 구성
- 모든 Entry 네이티브 블록에 대한 친숙한 별칭

위 기능은 현재 README에 설명된 CLI 및 언어 기능에 포함되지 않습니다. 향후 버전에서 구현될 수 있습니다.

---

## 참고 자료

- [Entry 사용자 위키](https://docs.playentry.org/user/)
- [Entry 블록 탭 및 카테고리](https://docs.playentry.org/user/tab_block.html)
- [EntryJS Project Data](https://docs.playentry.org/entryjs/typedef/2024-03-15-project-data.html)
- [EntryJS Variable Data](https://docs.playentry.org/entryjs/typedef/2024-03-15-variable-data.html)
- [EntryJS Function Data](https://docs.playentry.org/entryjs/typedef/2024-03-15-function-data.html)
- [EntryJS Object Data](https://docs.playentry.org/entryjs/typedef/2024-03-15-object-data.html)
- [EntryJS ENT 파일 관리](https://docs.playentry.org/entryjs/file/2024-07-24-ent.html)

## 라이선스

이 프로젝트의 라이선스는 저장소의 `Cargo.toml`에 명시된 MIT License를 따릅니다.
