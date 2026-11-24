# Entscript Specification

> **이 문서는 현재 저장소의 실제 구현을 기록한 명세다.** 미래의 설계 목표나 아직 구현되지 않은 기능을 포함하지 않는다.

- **프로젝트 버전:** `0.2.0`
- **Cargo package:** `entscript-converter`
- **CLI 실행 파일:** `entscript`
- **언어 파일 확장자:** `.ents`
- **Entry 프로젝트 패키지 확장자(관례):** `.ent`
- **Rust edition:** 2021
- **명세 기준:** 이 저장소에 포함된 소스 코드와 테스트

기능 상태는 다음과 같이 구분한다.

| 표기 | 의미 |
|---|---|
| 지원 | 현재 코드에서 직접 처리하고 테스트 또는 구현 근거가 확인됨 |
| 제한적 지원 | 일부 형식이나 상황만 처리하며, Entry 런타임 동작까지 보장하지 않음 |
| 미지원 | 현재 구현에 기능이 없음 |

---

## 1. 개요

Entscript는 텍스트로 작성한 스크립트를 Entry 프로젝트 JSON으로 변환하는 언어 및 컴파일러다.

```text
.ents source
    │
    ▼
Lexer
    │
    ▼
Parser ──► AST
    │
    ▼
Analyzer
    │
    ▼
EntryBuilder ──► Entry IR
    │
    ▼
Serializer
    │
    ▼
Entry project JSON
    │
    └── CLI 사용 시: gzip TAR 패키지(.ent)
```

라이브러리 API는 Entry 프로젝트 JSON 문자열을 반환한다. `.ent` 패키지 생성과 에셋 복사는 CLI가 담당한다.

### 1.1 기본 원칙

- 하나의 컴파일 요청에서 하나의 Entscript 소스 문자열을 파싱한다.
- 소스는 전역 변수·리스트 선언, 함수 선언, 오브젝트 이벤트 섹션으로 구성된다.
- 스크립트는 Entry의 네이티브 블록 JSON 구조로 직렬화된다.
- 기존 Entry 프로젝트 JSON을 입력하면 대상 오브젝트 스크립트를 교체하고, 나머지 프로젝트 데이터는 가능한 한 보존한다.
- 등록된 블록 별칭은 컴파일러가 파라미터 구조를 알고 생성한다. 미등록 일반 블록은 `raw_block()`으로 직접 구성할 수 있다.

---

## 2. 프로젝트 파일과 CLI

### 2.1 프로젝트 디렉터리

권장 구조는 다음과 같다.

```text
project/
├── settings.json
├── main.ents
├── modules/             # 현재 자동 컴파일되지 않는 보관용 디렉터리
│   └── player.ents
├── assets/              # Entry 프로젝트 JSON이 참조하는 실제 에셋
│   └── ab/cd/image/abcd1234.png
└── build/               # 출력 디렉터리 예시
```

`modules/`의 파일을 자동으로 탐색하거나 `main.ents`에 합치는 기능은 없다. 현재 빌드 모드는 `build.entry`에 지정된 단일 파일만 컴파일한다.

### 2.2 프로젝트 빌드 모드

```text
entscript build [OPTIONS]
```

| 옵션 | 기능 |
|---|---|
| `-C`, `--project-dir <DIR>` | 프로젝트 디렉터리 지정. 기본값은 현재 디렉터리 `.` |
| `--settings <FILE>` | 설정 파일 지정. 기본값은 `settings.json` |
| `-i`, `--entry <FILE>` | 설정의 `build.entry`를 덮어씀 |
| `-o`, `--output <FILE>` | 설정의 `build.output`을 덮어씀 |
| `--name <NAME>` | 설정의 `metadata.name`을 덮어씀 |
| `-p`, `--project <FILE>` | 기존 Entry 프로젝트 JSON을 사용 |
| `-h`, `--help` | 도움말 출력 |
| `-V`, `--version` | 버전 출력 |

예시:

```bash
entscript build
entscript build --project-dir ./project
entscript build --entry src/main.ents --output build/game.ent
entscript build --name "My Game"
entscript build --project existing-project.json
```

`--project-dir`의 상대 경로는 실행 당시 현재 디렉터리를 기준으로 절대화한다. 다음 경로들은 프로젝트 디렉터리를 기준으로 해석한다.

- `--settings`
- `build.entry`
- `build.output`
- `--entry`
- `--output`
- `--project`

CLI 옵션이 설정 파일보다 우선한다. 설정에서 생략된 값은 컴파일러 기본값을 사용한다.

- 기본 설정 파일: `settings.json`
- 기본 진입 파일: `main.ents`
- 기본 출력 파일: 진입 파일 경로의 확장자를 `.ent`로 바꾼 경로
- 빌드 모드에서 `entry: "-"`는 허용하지 않는다.
- 출력 경로 `-`는 gzip TAR 데이터를 표준 출력으로 내보낸다.

설정 파일이 존재하지 않으면 경고를 출력하고 기본 설정을 사용한다. 파일 읽기 오류나 잘못된 설정은 빌드 실패다.

### 2.3 직접 컴파일 모드

```text
entscript <SOURCE> [OPTIONS]
```

| 옵션 | 기능 |
|---|---|
| `<SOURCE>` | `.ents` 파일 경로 또는 `-`(표준 입력) |
| `-o`, `--output <FILE>` | 출력 경로 지정 |
| `-p`, `--project <FILE>` | 기존 Entry 프로젝트 JSON 지정 |
| `-h`, `--help` | 도움말 출력 |
| `-V`, `--version` | 버전 출력 |
| `--` | 이후 인자를 위치 인자로 처리 |

예시:

```bash
entscript hello.ents
entscript hello.ents -o output/game.ent
entscript hello.ents --project project.json -o updated.ent
cat hello.ents | entscript - -o - > hello.ent
```

동작 규칙:

- 파일 소스의 기본 출력은 입력 파일의 확장자를 `.ent`로 변경한 경로다.
- 소스가 `-`이고 출력 옵션을 생략하면 출력도 `-`로 처리한다.
- 직접 컴파일의 상대 소스·프로젝트·출력 경로는 현재 작업 디렉터리 기준이다.
- 직접 컴파일에서 에셋 루트는 현재 작업 디렉터리의 `assets/`다. 소스 파일이 있는 폴더를 기준으로 하지 않는다.

### 2.4 종료 코드

- 인자 파싱 오류: 프로세스 종료 코드 `2`
- 빌드 또는 컴파일 처리 오류: 프로세스 종료 코드 `1`
- 도움말과 버전 출력: 정상 종료

### 2.5 `settings.json` 명세

최상위 JSON 객체의 지원 필드는 다음과 같다.

```json
{
  "version": 1,
  "metadata": {
    "name": "My Project",
    "description": "Project description",
    "category": "game",
    "tags": ["example", "entscript"],
    "thumbnail": "assets/thumbnail.png"
  },
  "build": {
    "entry": "main.ents",
    "output": "build/project.ent"
  }
}
```

| 필드 | 자료형 | 필수 | 동작 |
|---|---|---:|---|
| `version` | 정수 | 예 | 현재 `1`만 허용 |
| `metadata` | 객체 | 아니요 | 작품 메타데이터 |
| `build` | 객체 | 아니요 | 입력·출력 경로 |
| `metadata.name` | 문자열 | 아니요 | 프로젝트 최상위 `name` 덮어쓰기 |
| `metadata.description` | 문자열 | 아니요 | 프로젝트 최상위 `description`에 기록 |
| `metadata.category` | 문자열 | 아니요 | 프로젝트 최상위 `category`에 기록 |
| `metadata.tags` | 문자열 배열 | 아니요 | 프로젝트 최상위 `tags`에 기록 |
| `metadata.thumbnail` | 문자열 또는 `null` | 아니요 | 입력 문자열 그대로 최상위 `thumbnail`에 기록. `null`은 기존 값 삭제 |
| `build.entry` | 비어 있지 않은 문자열 | 아니요 | 진입 파일. 기본값 `main.ents` |
| `build.output` | 비어 있지 않은 문자열 | 아니요 | 출력 파일. 생략 시 진입 파일 기반 기본 경로 |

유효성 규칙:

- `version`은 필수이며 JSON unsigned integer로 읽을 수 있는 `1`이어야 한다.
- JSON 최상위 값은 객체여야 한다.
- `metadata`, `build`는 존재하면 객체여야 한다.
- `metadata.name`, `description`, `category`는 문자열이어야 한다.
- `metadata.tags`는 문자열만 포함하는 배열이어야 한다.
- `metadata.thumbnail`만 문자열 또는 `null`을 허용한다. 다른 메타데이터 필드의 `null`은 허용하지 않는다.
- `build.entry`, `build.output`은 문자열이어야 하며 빈 문자열은 허용하지 않는다.
- `build.entry`에 `-`를 지정할 수 없다.
- 알 수 없는 필드는 경고를 출력하고 무시한다.
- 누락된 메타데이터는 기존 프로젝트 값 또는 기본 프로젝트 값을 유지한다.
- `--name`만 CLI에서 직접 메타데이터를 덮어쓸 수 있다. 다른 메타데이터는 설정 파일을 사용한다.

**주의:** `description`, `category`, `tags`, `thumbnail`은 현재 JSON 최상위에 기록될 뿐이다. Entry 편집기나 게시 화면에서 이 필드를 표시·사용하는지는 보장하지 않는다. `thumbnail` 문자열은 경로 해석, 로컬 파일 포함, 이미지 임베드, 서버 업로드를 하지 않는다.

---

## 3. 소스 언어 문법

### 3.1 최상위 구조

```entscript
var score = 0
list items = ["coin", "key"]

fn greet(name: string) {
    say(name)
}

@엔트리봇 {
    when start {
        greet("Hello")
    }
}
```

지원되는 최상위 선언은 네 종류다.

1. `var`: 스칼라 전역 변수
2. `list`: 전역 리스트
3. `fn`: 사용자 함수
4. `@오브젝트이름 { ... }`: 오브젝트 이벤트 스크립트 묶음

최상위 선언은 여러 개 쓸 수 있으며 서로 섞어 배치할 수 있다. 오브젝트 섹션은 프로젝트에 이미 존재하는 오브젝트를 대상으로 한다.

### 3.2 EBNF 개요

아래 문법은 파서 구조를 간략화한 문법이다. 별도 의미 분석 규칙은 뒤의 절을 함께 참조한다.

```ebnf
program             = { top_level } ;
top_level           = variable_decl
                    | list_decl
                    | function_decl
                    | object_section ;

variable_decl       = "var", identifier, "=", expression ;
list_decl           = "list", identifier, "=", "[",
                      [ expression, { ",", expression }, [ "," ] ], "]" ;

function_decl       = "fn", identifier, "(", [ parameter_list ], ")",
                      [ "->", expression ], block ;
parameter_list      = parameter, { ",", parameter }, [ "," ] ;
parameter           = identifier, [ ":", parameter_type ] ;
parameter_type      = "value" | "string" | "bool" | "boolean" ;

object_section      = "@", identifier, "{", { event_handler }, "}" ;
event_handler       = "when", identifier, [ "(", [ argument_list ], ")" ], block ;

block               = "{", { statement }, "}" ;
statement           = identifier,
                      [ "(", [ argument_list ], ")"
                      | expression
                      | block ],
                      [ "else", block ] ;
argument_list       = expression, { ",", expression }, [ "," ] ;

expression          = logical_or ;
logical_or          = logical_and, { "||", logical_and } ;
logical_and         = equality, { "&&", equality } ;
equality            = comparison, { ( "==" | "!=" ), comparison } ;
comparison          = term, { ( "<" | "<=" | ">" | ">=" ), term } ;
term                = factor, { ( "+" | "-" ), factor } ;
factor              = unary, { ( "*" | "/" | "%" ), unary } ;
unary               = ( "-" | "!" ), unary | primary ;
primary             = string | number | boolean | "null"
                    | identifier, [ "(", [ argument_list ], ")" ]
                    | "(", expression, ")" ;
```

위 EBNF의 `statement`는 파서의 편의 문법을 표현한 것이다. 실제로 괄호 없는 호출은 인자 표현식 하나만 받을 수 있다. 여러 인자는 괄호 형태를 사용한다. 의미 분석 단계에서 등록된 블록의 인자 수와 블록 영역을 추가 검사한다.

### 3.3 공백, 줄바꿈, 세미콜론

- 공백, 탭, 줄바꿈은 토큰 구분용 공백으로 처리한다.
- 줄바꿈 자체는 문장 종료 토큰이 아니다.
- 세미콜론 `;`은 지원하지 않는다. 사용하면 알 수 없는 문자 오류가 발생한다.
- 각 문장을 별도 줄에 쓰는 것을 권장하지만, 파서는 문장 구조를 토큰으로 구분한다.

### 3.4 주석

`//`부터 해당 줄의 끝까지 한 줄 주석으로 처리한다.

```entscript
// 초기 점수
var score = 0
```

블록 주석 `/* ... */`은 지원하지 않는다.

### 3.5 식별자

식별자 규칙:

- 첫 글자: 밑줄 `_` 또는 Unicode 알파벳 문자
- 다음 글자: 밑줄 `_`, Unicode 알파벳 문자 또는 숫자
- 한글 식별자를 사용할 수 있다.
- 대소문자를 구분한다.

```entscript
var player_score = 0
var 플레이어이름 = "Ada"
```

### 3.6 리터럴

| 종류 | 예시 |
|---|---|
| 정수 | `0`, `42`, `1000` |
| 소수 | `3.14`, `0.5` |
| 문자열 | `"hello"` |
| 참 | `true` |
| 거짓 | `false` |
| null | `null` |

숫자는 ASCII 십진 숫자를 사용한다. 소수점은 숫자 앞뒤에 숫자가 있어야 인식된다.

- 지수 표기법(예: `1e3`): 미지원
- 16진수 표기법: 미지원
- 숫자 리터럴 내 밑줄 구분자: 미지원
- 선행 부호 `+`는 리터럴 문법이 아니며 unary plus도 미지원
- 음수는 unary `-` 표현식으로 파싱된다.

### 3.7 문자열과 이스케이프

문자열은 큰따옴표 `"`로 감싼다. 지원하는 이스케이프는 아래 여섯 가지다.

| 표기 | 결과 |
|---|---|
| `\\n` | 줄바꿈 문자 |
| `\\r` | 캐리지 리턴 |
| `\\t` | 탭 |
| `\\0` | NUL 문자 |
| `\\"` | 큰따옴표 |
| `\\\\` | 역슬래시 |

이스케이프되지 않은 실제 줄바꿈은 문자열 안에서 허용되지 않는다. 위 목록 외의 이스케이프 시퀀스는 Lexer 오류다. 작은따옴표 문자열, 문자열 보간, raw string은 지원하지 않는다.

### 3.8 연산자와 우선순위

낮은 우선순위부터 높은 우선순위 순서다.

| 우선순위 | 연산자 | 의미 |
|---:|---|---|
| 1 | `||` | 논리 OR |
| 2 | `&&` | 논리 AND |
| 3 | `==`, `!=` | 동등 / 부등 |
| 4 | `<`, `<=`, `>`, `>=` | 크기 비교 |
| 5 | `+`, `-` | 덧셈 / 뺄셈 |
| 6 | `*`, `/`, `%` | 곱셈 / 나눗셈 / 나머지 |
| 7 | unary `-`, `!` | 부호 반전 / 논리 부정 |

같은 우선순위의 이항 연산자는 왼쪽 결합으로 파싱된다. 괄호로 계산 순서를 바꿀 수 있다.

```entscript
var result = 10 + 20 * 3

@엔트리봇 {
    when start {
        say((10 + 20) * 3)
        if score >= 10 && score < 100 {
            say("Level up")
        }
    }
}
```

Entry 변환:

| Entscript | 생성되는 Entry 타입/선택값 |
|---|---|
| `a + b` | `calc_basic`, `PLUS` |
| `a - b` | `calc_basic`, `MINUS` |
| `a * b` | `calc_basic`, `MULTI` |
| `a / b` | `calc_basic`, `DIVIDE` |
| `a % b` | `quotient_and_mod`, `MOD` |
| `a == b` | `boolean_basic_operator`, `EQUAL` |
| `a != b` | `boolean_basic_operator`, `NOT_EQUAL` |
| `a < b` | `boolean_basic_operator`, `LESS` |
| `a <= b` | `boolean_basic_operator`, `LESS_OR_EQUAL` |
| `a > b` | `boolean_basic_operator`, `GREATER` |
| `a >= b` | `boolean_basic_operator`, `GREATER_OR_EQUAL` |
| `a && b` | `boolean_and_or`, `AND` |
| `a || b` | `boolean_and_or`, `OR` |
| `!a` | `boolean_not` |
| `-a` | `calc_basic`의 `0 - a` |

문자열 연결 연산자로서 `+`의 동작은 별도 보장하지 않는다. 컴파일러는 `+`를 Entry 계산 블록의 `PLUS` 연산으로 변환한다.

### 3.9 표현식

표현식의 기본 요소는 다음과 같다.

- 문자열·숫자·불리언·`null` 리터럴
- 변수 또는 함수 매개변수 식별자
- 리포터 함수 호출 `name(arguments...)`
- 괄호식
- unary 및 binary 연산식

일반적인 Entry 값 블록 호출은 표현식 위치에서 사용할 수 있다.

```entscript
say(random(1, 10))
say(string_length("Entry"))
say(list_get(items, 1))
```

등록된 문장 블록은 표현식 값으로 사용할 수 없다. 문장 함수도 표현식 값으로 사용할 수 없다. 값 함수는 표현식 위치에서 사용할 수 있다.

---

## 4. 변수와 리스트

### 4.1 전역 스칼라 변수 선언

```entscript
var score = 0
var player_name = "Ada"
var enabled = true
var empty_value = null
```

선언은 최상위에서만 가능하다. 새 변수는 Entry 프로젝트의 전역 변수로 추가된다.

초기값은 아래 네 종류의 리터럴만 허용한다.

- 문자열
- 숫자
- 불리언
- `null`

계산식, 다른 변수, 함수 호출은 초기값으로 사용할 수 없다. unary `-`는 AST에서 별도 unary 표현식이므로 음수 초기값도 현재 literal 검사에 통과하지 않는다. 음수는 스크립트의 일반 표현식이나 `raw()` 등에서 사용할 수 있다.

생성되는 변수 데이터의 주요 값:

- `variableType: "variable"`
- `object: null` (전역)
- `value`: 리터럴을 문자열로 변환한 값. `null`은 빈 문자열
- `visible: false`
- `isCloud: false`
- `isRealTime: false`
- `cloudDate: false`
- 표시 좌표 기본값 `x: 150`, `y: -100`

### 4.2 변수 읽기와 쓰기

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

| Entscript | Entry 블록 | 인자 |
|---|---|---|
| `set(name, value)` / `set_variable(name, value)` | `set_variable` | 변수 이름, 값 |
| `change(name, value)` / `change_variable(name, value)` | `change_variable` | 변수 이름, 변화량 |
| `show_variable(name)` | `show_variable` | 변수 이름 |
| `hide_variable(name)` | `hide_variable` | 변수 이름 |
| `get_variable(name)` | `get_variable` 리포터 | 변수 이름 |

변수 이름은 `set`, `change`, `show_variable`, `hide_variable`, `get_variable`에서 식별자 또는 문자열 리터럴로 지정할 수 있다. 단, `get_variable()`이 인자 없는 기본 리포터 형태로 사용되는 경우는 등록된 Entry 리포터 사양에 따라 생성된다.

### 4.3 기존 프로젝트 변수 참조

기존 프로젝트 JSON의 변수도 이름으로 참조할 수 있다. 프로젝트에서 지원하는 `variableType` 값은 검증 단계에서 다음으로 제한된다.

- `variable`
- `list`
- `timer`
- `answer`
- `slide`

스칼라 변수 사용 시 `variableType == "list"`인 항목은 거부된다. `timer`, `answer`, `slide`는 리스트가 아니므로 스칼라 변수 API 경로로 처리될 수 있다.

오브젝트 이벤트 섹션 내부에서 이름을 해석할 때:

1. 해당 오브젝트의 지역 변수/리스트
2. 전역 변수/리스트

순서로 찾는다. 같은 이름의 지역 항목이 있으면 전역 항목보다 우선한다. 같은 범위에서 이름이 모호하면 오류다.

함수 본문은 오브젝트 범위가 없으므로 오브젝트 지역 변수를 이름으로 찾지 않는다. 전역 변수와 함수 매개변수만 대상으로 한다.

Entscript에서 `var` 또는 `list`로 새로 선언하는 항목은 전역이다. 지역 변수 선언 문법은 없다.

### 4.4 리스트 선언

```entscript
list inventory = ["coin", "key"]
list scores = [10, 20, 30]
list empty_list = []
```

리스트 항목은 문자열·숫자·불리언·`null` 리터럴만 허용한다. 계산식과 식별자는 허용하지 않는다. 마지막 쉼표는 허용된다.

생성되는 리스트 데이터의 주요 구조:

```json
{
  "name": "inventory",
  "variableType": "list",
  "array": [
    { "data": "coin" },
    { "data": "key" }
  ]
}
```

각 리스트 값은 Entry의 `data` 문자열로 저장된다. `null` 항목은 빈 문자열로 변환된다.

### 4.5 리스트 연산

문장 블록:

| Entscript | 인자 | Entry 네이티브 타입 |
|---|---|---|
| `list_add(list, value)` | 리스트, 추가 값 | `add_value_to_list` |
| `list_insert(list, index, value)` | 리스트, 위치, 값 | `insert_value_to_list` |
| `list_set(list, index, value)` | 리스트, 위치, 값 | `change_value_list_index` |
| `list_remove(list, index)` | 리스트, 위치 | `remove_value_from_list` |
| `list_show(list)` | 리스트 | `show_list` |
| `list_hide(list)` | 리스트 | `hide_list` |

리포터:

| Entscript | 인자 | Entry 네이티브 타입 |
|---|---|---|
| `list_get(list, index)` | 리스트, 위치 | `value_of_index_from_list` |
| `list_length(list)` | 리스트 | `length_of_list` |
| `list_contains(list, value)` | 리스트, 값 | `is_included_in_list` |

리스트 이름은 식별자 또는 문자열 리터럴로 전달할 수 있다. 인덱스는 Entry와 동일하게 1부터 시작한다. 인덱스 범위 검증이나 자동 보정은 컴파일러가 하지 않는다. 실제 범위 동작은 Entry 런타임에 따른다.

---

## 5. 함수

### 5.1 함수 선언

```entscript
fn greet(name) {
    say(name)
}

@엔트리봇 {
    when start {
        greet("Hello")
    }
}
```

함수는 최상위에서만 선언한다. 함수 이름과 매개변수는 식별자 규칙을 따른다.

### 5.2 매개변수

```entscript
fn print_value(value) {
    say(value)
}

fn check(message: string, condition: bool) {
    if condition {
        say(message)
    }
}
```

| 문법 | 내부 종류 | Entry 함수 매개변수 블록 |
|---|---|---|
| 타입 생략 | 값 | `function_field_string` |
| `: value` | 값 | `function_field_string` |
| `: string` | 값 | `function_field_string` |
| `: bool` | 불리언 | `function_field_boolean` |
| `: boolean` | 불리언 | `function_field_boolean` |

`value`와 `string`은 동일한 값 매개변수 종류다. 그 이름이 문자열만 허용한다는 뜻은 아니다. 현재 매개변수 종류는 값과 불리언 두 가지다.

### 5.3 문장 함수와 값 함수

반환 표현식이 없는 함수는 문장 함수다.

```entscript
fn announce(message) {
    say(message)
}
```

`->` 뒤에 반환 표현식을 쓰면 값 함수가 된다.

```entscript
fn add(a, b) -> a + b {
}

@엔트리봇 {
    when start {
        say(add(10, 20))
    }
}
```

- 반환값은 `->` 뒤의 표현식으로만 지정한다.
- `return` 문은 존재하지 않는다.
- 값 함수의 반환 표현식은 리포터/값으로 빌드할 수 있는 표현식이어야 한다.
- 문장 함수는 문장 위치에서 호출한다.
- 값 함수는 표현식 위치에서 호출한다.
- 값 함수를 문장으로 호출하거나 문장 함수를 값으로 쓰면 오류다.

### 5.4 함수 호출과 검증

- 사용자 함수의 인자 개수는 정확히 일치해야 한다.
- 전방 호출을 지원한다. 소스에서 나중에 선언된 함수도 앞 함수에서 호출할 수 있다.
- 함수 ID를 먼저 할당하므로 자기 자신을 호출하거나 함수끼리 참조할 수 있는 구조다.
- 함수 이름과 매개변수 이름의 중복은 허용하지 않는다.
- 함수 이름은 등록된 내장 문장/리포터 별칭 및 일부 특수 함수 이름과 충돌할 수 없다.
- 기존 프로젝트에 이미 같은 이름의 사용자 함수가 있으면 새 함수 선언은 거부된다.

불리언 인자 검사는 제한적인 정적 추론이다. `bool` 매개변수에 명확히 값으로 알려진 표현식을 전달하거나 값 매개변수에 명확히 불리언인 표현식을 전달하면 오류가 될 수 있다. 모든 리포터와 모든 Entry 값의 자료형을 완전하게 추론하는 타입 시스템은 아니다.

### 5.5 기존 Entry 함수 호출

기존 프로젝트의 함수를 ID로 직접 호출할 수 있다.

```entscript
@엔트리봇 {
    when start {
        func_ab12("hello", true)
        say(func_cd34(10))
    }
}
```

- `func_<ID>(...)` 형식을 사용한다.
- `functions` 배열에서 해당 ID가 존재해야 한다.
- 함수 데이터의 `type`이 `normal`이면 문장 함수, `value`이면 값 함수로 취급한다.
- 함수의 `content` JSON에서 함수 정의 블록과 매개변수 리본을 읽어 인자 개수 및 확인 가능한 값/불리언 타입을 검사한다.
- 프로젝트 함수의 매개변수 정의를 해석할 수 없으면 프로젝트 검증 단계에서 오류가 발생한다.

### 5.6 미지원 함수 기능

- 함수 오버로딩
- 기본 인자값
- 가변 인자
- 이름 있는 인자 호출
- 함수 내부 지역 변수 선언
- 참조/포인터/출력 매개변수
- 함수 반환 타입을 별도 타입 문법으로 선언하는 기능
- 함수 자체를 값으로 전달하는 기능
- 람다/익명 함수
- 중첩 함수 선언

새로 생성되는 함수는 `useLocalVariables: false`, `localVariables: []`로 저장된다. 함수 지역 변수 시스템을 생성하거나 활성화하지 않는다.

---

## 6. 오브젝트와 이벤트

### 6.1 오브젝트 섹션

```entscript
@엔트리봇 {
    when start {
        say("시작")
    }
}
```

- `@` 뒤에는 Entry 프로젝트에 실제 존재하는 오브젝트 이름이 온다.
- 이름은 대소문자 및 문자까지 정확히 일치해야 한다.
- 해당 이름의 오브젝트가 없거나 두 개 이상이면 오류다.
- 같은 오브젝트 이름을 소스에서 두 번 선언하면 오류다.
- 한 오브젝트 섹션 안에는 여러 `when` 핸들러를 쓸 수 있다.
- 여러 오브젝트 섹션을 한 소스에 둘 수 있다.

기본 프로젝트 템플릿에는 `엔트리봇` 오브젝트가 포함되어 있다. 다른 오브젝트는 `--project`로 기존 프로젝트를 전달해야 한다.

### 6.2 이벤트 문법

```entscript
@엔트리봇 {
    when start {
        say("시작 버튼")
    }

    when key("81") {
        say("키 입력")
    }

    when mouse_click {
        say("마우스 클릭")
    }
}
```

문법은 `when 이벤트이름 [(인자들)] { 문장들 }`이다. 공인 별칭 또는 Entry 네이티브 이벤트 타입 ID를 사용할 수 있다.

### 6.3 등록된 이벤트 별칭

| Entscript 이벤트 | Entry 타입 |
|---|---|
| `start` | `when_run_button_click` |
| `key`, `key_pressed` | `when_some_key_pressed` |
| `mouse_click` | `mouse_clicked` |
| `mouse_release` | `mouse_click_cancled` |
| `object_click` | `when_object_click` |
| `object_release` | `when_object_click_canceled` |
| `message` | `when_message_cast` |
| `scene_start` | `when_scene_start` |
| `clone_start` | `when_clone_start` |

다음 네이티브 ID도 등록된 이벤트 사양으로 인식한다.

- `when_run_button_click`
- `when_some_key_pressed`
- `mouse_clicked`
- `mouse_click_cancled`
- `when_object_click`
- `when_object_click_canceled`
- `when_scene_start`
- `when_clone_start`
- `when_message_cast`

등록 이벤트의 입력 개수는 레지스트리에 정의된 필수/선택 파라미터에 따라 검사된다. 예를 들어 `key`는 키 값 입력을 생략하면 기본값을 쓰며, `message`는 메시지 입력을 요구한다.

등록되지 않은 이벤트 이름은 네이티브 이벤트 ID로 간주해 블록을 만들려고 시도한다. 이 경우 컴파일러는 해당 네이티브 타입의 파라미터 개수·종류를 검증하지 않는다. AI/확장 계열로 분류되는 일부 ID 접두사는 거부된다.

---

## 7. 문장 블록과 블록 영역

### 7.1 일반 호출

일반 문장 블록은 함수 호출 형식으로 쓴다.

```entscript
say("Hello")
wait(1)
move(10)
show()
hide()
```

인자가 없는 호출도 `show()`처럼 괄호를 쓰는 것을 권장한다.

### 7.2 내부 문장 영역

블록이 제어 영역을 갖는 경우 중괄호를 사용한다.

```entscript
for 10 {
    move(10)
    wait(0.2)
}

if score > 10 {
    say("high")
} else {
    say("low")
}
```

`if`에 `else`가 있으면 `if_else` Entry 타입으로 생성한다. `if`에 `else`가 없으면 `_if` 타입이다.

- 블록에 문장 영역이 없는데 `{}`를 붙이면 오류다.
- 두 번째 영역이 없는 블록에 `else {}`를 붙이면 오류다.
- 영역 개수는 등록된 블록 정의에 의해 결정된다.
- `raw_block()`은 개발자가 제공한 영역을 그대로 구성하며 일반 별칭 수준의 영역 검증을 하지 않는다.

### 7.3 인자 표기

- 여러 인자는 `name(arg1, arg2)` 형태로 쓴다.
- 파서의 축약 문법으로 `for 10 { ... }`, `if condition { ... }`처럼 괄호 없는 단일 인자 문장을 쓸 수 있다.
- 괄호 없는 문장은 인자 표현식 하나만 받을 수 있다.
- 이벤트 인자도 괄호 안에 쉼표로 구분한다.
- 등록된 블록은 최소/최대 인자 개수를 검사한다. 기본값이 정의된 입력 슬롯은 생략할 수 있다.

### 7.4 `field()`와 `raw()`

이 두 함수는 일반 Entry 리포터가 아니라 블록 입력 인코딩을 직접 지정하기 위한 컴파일러 내장 도구다.

```entscript
change_layer(field("front"))
raw_block("some_native_type", field("MODE"), raw(12), null)
```

| 표기 | 컴파일 동작 |
|---|---|
| `field("VALUE")` | 필드/드롭다운 문자열을 가공 없이 전달 |
| `field(identifier)` | 식별자 문자열을 필드 값으로 전달 |
| `raw("text")` | 원시 JSON 문자열 값 |
| `raw(12)` | 원시 JSON 숫자 값 |
| `raw(true)` / `raw(false)` | 원시 JSON 불리언 값 |
| `raw(null)` | 빈 Entry 입력(`null`) |
| `raw(-12)` | 원시 음수 숫자 값 |
| `null` | 빈 Entry 입력 슬롯 |
| 일반 문자열 | Entry `text` 값 블록 |
| 일반 숫자 | Entry `number` 값 블록 |
| 일반 불리언 | Entry `True`/`False` 값 블록 |

`raw()`는 정확히 하나의 문자열·숫자·불리언·`null` 리터럴 또는 음수 숫자를 받는다. 변수나 계산식을 원시값으로 변환할 수 없다.

`field()`는 정확히 하나의 문자열 또는 식별자를 받는다.

---

## 8. 네이티브 블록 접근

### 8.1 `raw_block()` / `entry_block()`

등록된 별칭이 없는 일반 Entry 블록을 타입 ID로 직접 구성한다.

```entscript
@엔트리봇 {
    when start {
        raw_block("some_native_type", field("DROPDOWN"), raw(12), null)
    }
}
```

`entry_block()`은 `raw_block()`과 동일한 기능을 가진 별칭이다.

첫 인자는 반드시 Entry 블록 타입 ID 문자열이어야 한다. 이후 인자는 Entry의 `params` 배열에 순서대로 들어간다.

`raw_block()`은 파라미터 슬롯의 정의를 자동으로 조회하거나 개수·인코딩을 검증하지 않는다. 따라서 다음을 사용자가 Entry 블록 정의와 대조해야 한다.

1. 정확한 타입 ID
2. `params`의 개수
3. 각 인자의 위치
4. 필드 문자열과 값 블록/원시 JSON 값 구분
5. `statements` 영역 개수와 순서

문장으로 사용하면 뒤에 `{ ... }` 및 선택적으로 `else { ... }`를 붙일 수 있다. 표현식 위치에서도 `raw_block("reporter_type", ...)`를 중첩 리포터로 만들 수 있지만, 타입이 실제 리포터인지 컴파일러가 확인하지 않는다.

### 8.2 직접 네이티브 ID 사용 규칙

| 위치 | 동작 |
|---|---|
| `when native_event_id {}` | 등록 이벤트 사양이면 파라미터 검증, 미등록 ID면 원시 이벤트 블록 생성 |
| 문장 `native_statement_id(...)` | 등록된 문장 레지스트리 항목만 직접 이름으로 인식 |
| 리포터 `native_reporter_id(...)` | 등록된 리포터 레지스트리 항목만 직접 이름으로 인식 |
| `raw_block("type_id", ...)` | 등록 여부와 무관하게 일반 네이티브 블록을 생성하려고 시도 |

등록되지 않은 일반 문장 타입 ID를 `raw_block()` 없이 문장 이름처럼 직접 호출하면 `Unknown block` 오류가 발생한다.

### 8.3 AI 및 확장 블록 제한

컴파일러는 다음 패턴에 해당하는 블록 타입 ID를 거부한다.

- `block_expansion_` 접두사
- `expansion_` 접두사
- `block_ai_` 접두사
- `ai_` 접두사
- `ai_utilize` 문자열 포함

이는 문자열 패턴을 기준으로 한 필터다. 단순히 임의 ID를 전달한다고 AI/확장 런타임을 사용할 수 있는 것은 아니다.

하드웨어 런타임, 외부 모듈 연결, AI 모델 설정, 확장 모듈 설치·초기화는 구현되어 있지 않다. `raw_block()`으로 어떤 네이티브 ID가 JSON에 기록되는 것과 Entry에서 실제로 작동하는 것은 별개의 문제다.

---

## 9. 내장 블록 레지스트리 전체 목록

아래 목록은 `src/entry/registry.rs`와 특수 처리 분기를 기준으로 정리한 현재 등록 이름이다. 이름은 대소문자를 구분한다. 별칭이 같은 Entry 타입으로 매핑되더라도 파라미터 기본값이나 의미가 완전히 같다고 단정할 수는 없다.

### 9.1 특수 처리 블록 및 컴파일러 도구

| Entscript 이름 | 구분 | 설명 |
|---|---|---|
| `set`, `set_variable` | 문장 | 변수 값 설정 |
| `change`, `change_variable` | 문장 | 변수 값 변경 |
| `show_variable` | 문장 | 변수 표시 |
| `hide_variable` | 문장 | 변수 숨김 |
| `get_variable` | 리포터 | 변수 값 가져오기 |
| `list_add` | 문장 | 리스트 끝에 추가 |
| `list_insert` | 문장 | 리스트 위치에 삽입 |
| `list_set` | 문장 | 리스트 항목 변경 |
| `list_remove` | 문장 | 리스트 항목 삭제 |
| `list_show` | 문장 | 리스트 표시 |
| `list_hide` | 문장 | 리스트 숨김 |
| `list_get` | 리포터 | 리스트 항목 읽기 |
| `list_length` | 리포터 | 리스트 길이 |
| `list_contains` | 리포터 | 리스트 포함 여부 |
| `field` | 입력 도구 | 원시 필드값 지정 |
| `raw` | 입력 도구 | 원시 JSON 값 지정 |
| `raw_block` | 네이티브 블록 도구 | 블록 타입 ID로 직접 생성 |
| `entry_block` | 네이티브 블록 도구 | `raw_block`과 동일 |

### 9.2 이벤트 이름

| Entscript 별칭 | Entry 타입 |
|---|---|
| `start` | `when_run_button_click` |
| `key`, `key_pressed` | `when_some_key_pressed` |
| `mouse_click` | `mouse_clicked` |
| `mouse_release` | `mouse_click_cancled` |
| `object_click` | `when_object_click` |
| `object_release` | `when_object_click_canceled` |
| `message` | `when_message_cast` |
| `scene_start` | `when_scene_start` |
| `clone_start` | `when_clone_start` |

등록된 네이티브 이벤트 ID: `when_run_button_click`, `when_some_key_pressed`, `mouse_clicked`, `mouse_click_cancled`, `when_object_click`, `when_object_click_canceled`, `when_scene_start`, `when_clone_start`, `when_message_cast`.

### 9.3 흐름 및 제어 문장 블록

| Entscript 이름 | Entry 타입 |
|---|---|
| `wait`, `wait_second` | `wait_second` |
| `for`, `repeat`, `repeat_basic` | `repeat_basic` |
| `loop`, `repeat_inf` | `repeat_inf` |
| `repeat_until`, `repeat_until_true`, `repeat_while_true`, `repeat_while` | `repeat_while_true` |
| `break`, `stop_repeat` | `stop_repeat` |
| `continue`, `continue_repeat` | `continue_repeat` |
| `if`, `_if` | `_if` |
| `if_else` | `if_else` |
| `wait_until`, `wait_until_true` | `wait_until_true` |
| `stop`, `stop_object`, `stop_all`, `stop_all_scripts`, `stop_this_object`, `stop_this_thread`, `stop_other_threads`, `stop_other_objects` | `stop_object` |
| `restart`, `restart_project` | `restart_project` |
| `send_message`, `message_cast` | `message_cast` |
| `send_message_and_wait`, `message_cast_wait` | `message_cast_wait` |
| `start_scene` | `start_scene` |
| `start_neighbor_scene` | `start_neighbor_scene` |
| `create_clone` | `create_clone` |
| `delete_clone` | `delete_clone` |
| `delete_all_clones`, `remove_all_clones` | `remove_all_clones` |

### 9.4 생김새 및 대화 블록

| Entscript 이름 | Entry 타입 |
|---|---|
| `say`, `dialog` | `dialog` |
| `say_for`, `dialog_time` | `dialog_time` |
| `think_for` | `dialog_time` |
| `think` | `dialog` |
| `clear_dialog`, `remove_dialog` | `remove_dialog` |
| `show` | `show` |
| `hide` | `hide` |
| `ask`, `ask_and_wait` | `ask_and_wait` |
| `show_answer`, `hide_answer`, `set_visible_answer`, `answer_visibility` | `set_visible_answer` |
| `next_shape`, `change_to_next_shape`, `previous_shape`, `prev_shape` | `change_to_next_shape` |
| `change_shape`, `change_to_some_shape` | `change_to_some_shape` |
| `erase_effects`, `erase_all_effects` | `erase_all_effects` |
| `add_effect`, `add_effect_amount` | `add_effect_amount` |
| `set_effect`, `change_effect`, `change_effect_amount` | `change_effect_amount` |
| `change_size`, `add_size`, `change_scale_size` | `change_scale_size` |
| `set_size`, `set_scale_size` | `set_scale_size` |
| `stretch_size`, `stretch_scale_size` | `stretch_scale_size` |
| `reset_size`, `reset_scale_size` | `reset_scale_size` |
| `flip_x` | `flip_x` |
| `flip_y` | `flip_y` |
| `change_layer`, `change_object_index` | `change_object_index` |

### 9.5 프로젝트 타이머 및 대답 표시 블록

| Entscript 이름 | Entry 타입 |
|---|---|
| `timer_action`, `choose_project_timer_action` | `choose_project_timer_action` |
| `timer_start`, `start_timer` | `choose_project_timer_action` |
| `timer_stop`, `pause_timer` | `choose_project_timer_action` |
| `timer_reset`, `reset_timer` | `choose_project_timer_action` |
| `set_project_timer_visibility`, `set_visible_project_timer` | `set_visible_project_timer` |
| `show_project_timer` | `set_visible_project_timer` |
| `hide_project_timer` | `set_visible_project_timer` |

### 9.6 움직임 블록

| Entscript 이름 | Entry 타입 |
|---|---|
| `move`, `move_direction` | `move_direction` |
| `bounce`, `bounce_wall` | `bounce_wall` |
| `change_x`, `move_x` | `move_x` |
| `change_y`, `move_y` | `move_y` |
| `set_x`, `locate_x` | `locate_x` |
| `set_y`, `locate_y` | `locate_y` |
| `go_to_xy`, `locate_xy` | `locate_xy` |
| `go_to`, `locate`, `move_to_object` | `locate` |
| `go_to_mouse`, `move_to_mouse` | `locate` |
| `set_direction`, `direction_absolute` | `direction_absolute` |
| `turn`, `rotate_relative` | `rotate_relative` |
| `turn_direction`, `change_direction`, `direction_relative` | `direction_relative` |
| `rotate_absolute`, `set_rotation` | `rotate_absolute` |
| `rotate_by_time` | `rotate_by_time` |
| `direction_relative_duration`, `rotate_direction_by_time` | `direction_relative_duration` |
| `move_xy_time`, `move_xy_for` | `move_xy_time` |
| `locate_xy_time`, `locate_xy_for` | `locate_xy_time` |
| `locate_object_time`, `go_to_object_for` | `locate_object_time` |
| `see_angle_object`, `point_towards` | `see_angle_object` |
| `point_towards_mouse` | `see_angle_object` |
| `move_to_angle`, `move_direction_distance` | `move_to_angle` |

### 9.7 붓 블록

| Entscript 이름 | Entry 타입 |
|---|---|
| `stamp`, `brush_stamp` | `brush_stamp` |
| `start_drawing` | `start_drawing` |
| `stop_drawing` | `stop_drawing` |
| `start_fill` | `start_fill` |
| `stop_fill` | `stop_fill` |
| `set_brush_color`, `set_color` | `set_color` |
| `set_random_color`, `random_brush_color` | `set_random_color` |
| `set_fill_color`, `set_paint_color` | `set_fill_color` |
| `change_thickness`, `change_brush_thickness` | `change_thickness` |
| `set_thickness`, `set_brush_thickness` | `set_thickness` |
| `change_brush_transparency`, `change_brush_opacity` | `change_brush_transparency` |
| `set_brush_transparency`, `set_brush_tranparency` | `set_brush_tranparency` |
| `clear_brush`, `erase_all_brush`, `brush_erase_all` | `brush_erase_all` |

### 9.8 글상자 블록

| Entscript 이름 | Entry 타입 |
|---|---|
| `text_write`, `write_text` | `text_write` |
| `text_append`, `append_text` | `text_append` |
| `text_prepend`, `prepend_text` | `text_prepend` |
| `text_change_effect`, `set_text_effect` | `text_change_effect` |
| `text_change_font`, `set_text_font` | `text_change_font` |
| `text_change_font_color`, `set_text_color` | `text_change_font_color` |
| `text_change_bg_color`, `set_text_background` | `text_change_bg_color` |
| `text_flush`, `flush_text` | `text_flush` |

일부 글꼴 선택값은 Entry 편집기에서 동적으로 제공된다. 컴파일러가 설치된 글꼴 목록이나 런타임 글상자 객체를 관리하지 않는다.

### 9.9 소리 블록

| Entscript 이름 | Entry 타입 |
|---|---|
| `play_sound`, `sound_something_with_block` | `sound_something_with_block` |
| `play_sound_for`, `sound_something_second_with_block` | `sound_something_second_with_block` |
| `play_sound_from_to`, `sound_from_to` | `sound_from_to` |
| `play_sound_until_done`, `sound_something_wait_with_block` | `sound_something_wait_with_block` |
| `play_sound_for_and_wait`, `sound_something_second_wait_with_block` | `sound_something_second_wait_with_block` |
| `play_sound_from_to_and_wait`, `sound_from_to_and_wait` | `sound_from_to_and_wait` |
| `change_sound_volume`, `sound_volume_change` | `sound_volume_change` |
| `set_sound_volume`, `sound_volume_set` | `sound_volume_set` |
| `stop_all_sounds` | `sound_silent_all` |
| `stop_sounds`, `sound_silent_all` | `sound_silent_all` |
| `stop_this_object_sounds` | `sound_silent_all` |
| `stop_other_objects_sounds` | `sound_silent_all` |
| `play_bgm` | `play_bgm` |
| `stop_bgm` | `stop_bgm` |
| `change_sound_speed`, `sound_speed_change` | `sound_speed_change` |
| `set_sound_speed`, `sound_speed_set` | `sound_speed_set` |

### 9.10 데이터 분석 테이블 문장 블록

| Entscript 이름 | Entry 타입 |
|---|---|
| `table_append`, `append_table_row`, `append_row_to_table` | `append_row_to_table` |
| `table_insert`, `insert_table_row`, `insert_row_to_table` | `insert_row_to_table` |
| `table_delete`, `delete_table_row`, `delete_row_from_table` | `delete_row_from_table` |
| `table_set`, `set_table_value`, `set_value_from_table` | `set_value_from_table` |
| `table_save`, `preserve_table`, `save_current_table` | `save_current_table` |
| `table_open`, `open_table` | `open_table` |
| `table_open_for`, `open_table_wait` | `open_table_wait` |
| `table_open_chart`, `open_table_chart` | `open_table_chart` |
| `table_close_chart`, `close_table_chart` | `close_table_chart` |
| `table_set_cell`, `set_value_from_cell` | `set_value_from_cell` |

### 9.11 상태·좌표·입력 리포터

| Entscript 이름 | Entry 타입 |
|---|---|
| `mouse_x` | `coordinate_mouse` |
| `mouse_y` | `coordinate_mouse` |
| `clicked`, `is_clicked` | `is_clicked` |
| `is_boost_mode` | `is_boost_mode` |
| `get_project_timer_value`, `timer`, `stopwatch` | `get_project_timer_value` |
| `is_object_clicked` | `is_object_clicked` |
| `key_down`, `is_press_some_key` | `is_press_some_key` |
| `touching`, `reach_something` | `reach_something` |
| `object_x`, `x_of_object` | `coordinate_object` |
| `object_y`, `y_of_object` | `coordinate_object` |
| `object_rotation`, `rotation_of_object` | `coordinate_object` |
| `object_direction`, `direction_of_object` | `coordinate_object` |
| `object_size`, `size_of_object` | `coordinate_object` |
| `object_picture_index`, `picture_index_of_object` | `coordinate_object` |
| `object_picture_name`, `picture_name_of_object` | `coordinate_object` |
| `object_property`, `coordinate_object` | `coordinate_object` |

### 9.12 수학·불리언 리포터

| Entscript 이름 | Entry 타입 |
|---|---|
| `random`, `rand`, `calc_rand` | `calc_rand` |
| `calc_basic` | `calc_basic` |
| `boolean_basic_operator` | `boolean_basic_operator` |
| `boolean_and_or` | `boolean_and_or` |
| `boolean_not` | `boolean_not` |
| `sqrt`, `root`, `math_sqrt` | `calc_operation` |
| `square` | `calc_operation` |
| `abs`, `absolute` | `calc_operation` |
| `floor` | `calc_operation` |
| `ceil` | `calc_operation` |
| `round` | `calc_operation` |
| `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `log10`, `ln`, `factorial` | `calc_operation` |
| `calc_operation` | `calc_operation` |
| `quotient` | `quotient_and_mod` |
| `remainder`, `modulo` | `quotient_and_mod` |
| `fractional_part`, `decimal_part` | `calc_operation` |
| `distance`, `distance_to`, `distance_something` | `distance_something` |

### 9.13 문자열·텍스트 리포터

| Entscript 이름 | Entry 타입 |
|---|---|
| `text_read`, `read_text`, `text_box_value` | `text_read` |
| `string_length`, `length_of_string` | `length_of_string` |
| `reverse_string`, `reverse_of_string` | `reverse_of_string` |
| `combine`, `combine_text`, `combine_something` | `combine_something` |
| `char_at`, `string_char_at` | `char_at` |
| `substring`, `string_slice` | `substring` |
| `index_of`, `index_of_string`, `string_index_of` | `index_of_string` |
| `replace_string`, `string_replace` | `replace_string` |
| `uppercase` | `change_string_case` |
| `lowercase` | `change_string_case` |
| `change_string_case` | `change_string_case` |
| `count_match_string`, `count_substring` | `count_match_string` |

### 9.14 사용자·환경·날짜·색상 리포터

| Entscript 이름 | Entry 타입 |
|---|---|
| `get_user_name`, `username`, `user_id` | `get_user_name` |
| `block_count`, `get_block_count` | `get_block_count` |
| `rgb_to_hex`, `change_rgb_to_hex` | `change_rgb_to_hex` |
| `hex_to_rgb`, `change_hex_to_rgb` | `change_hex_to_rgb` |
| `boolean_value`, `get_boolean_value` | `get_boolean_value` |
| `current_device_type`, `is_current_device_type` | `is_current_device_type` |
| `get_nickname`, `nickname` | `get_nickname` |
| `get_date`, `date_part` | `get_date` |
| `is_type`, `is_value_type` | `is_type` |

### 9.15 테이블·프로젝트 데이터 리포터

| Entscript 이름 | Entry 타입 |
|---|---|
| `table_count`, `get_table_count` | `get_table_count` |
| `table_value`, `get_value_from_table` | `get_value_from_table` |
| `table_last_value`, `get_value_from_last_row` | `get_value_from_last_row` |
| `table_stat`, `table_statistics`, `calc_values_from_table` | `calc_values_from_table` |
| `table_correlation`, `get_coefficient` | `get_coefficient` |
| `table_cell`, `get_value_from_cell` | `get_value_from_cell` |
| `table_lookup`, `get_value_v_lookup` | `get_value_v_lookup` |
| `sound`, `get_sounds` | `get_sounds` |
| `picture`, `get_pictures` | `get_pictures` |
| `get_canvas_input_value`, `get_answer`, `answer` | `get_canvas_input_value` |
| `get_sound_volume` | `get_sound_volume` |
| `get_sound_speed` | `get_sound_speed` |
| `get_sound_duration` | `get_sound_duration` |

테이블 리포터 및 문장 블록은 기존 프로젝트의 `tables` 메타데이터에 포함된 ID를 사용하는 것을 전제로 한다. 현재 분석기는 테이블 ID의 존재나 테이블 열 이름을 사전 검증하지 않는다.

---

## 10. 기존 Entry 프로젝트 통합

### 10.1 컴파일 API/CLI

기존 프로젝트 JSON을 유지하면서 Entscript를 반영하려면 다음 중 하나를 사용한다.

```bash
entscript main.ents --project project.json -o updated.ent
entscript build --project existing-project.json
```

라이브러리 API:

- `compile_with_project_json(source, project_json)`
- `compile_with_project_value(source, project_value)`

### 10.2 교체와 보존 범위

- 소스의 각 `@오브젝트이름`에 대응하는 프로젝트 오브젝트의 `script` 필드는 새 스크립트로 교체된다.
- 대상 오브젝트의 기존 스크립트는 블록 단위로 병합되지 않는다.
- 지정하지 않은 오브젝트의 스크립트는 수정하지 않는다.
- 기존 프로젝트 변수·리스트·함수·장면·테이블 및 다른 미대상 데이터는 별도 수정 대상이 아닌 한 보존된다.
- 소스에서 새로 선언한 변수/리스트는 프로젝트의 `variables` 배열에 추가된다.
- 소스에서 새로 선언한 함수는 프로젝트의 `functions` 배열에 추가된다.
- 기존 프로젝트 함수·변수와 이름 충돌을 검사한다.
- Entry 프로젝트 최상위 JSON에서 `objects` 키가 가장 먼저 직렬화되도록 순서를 조정한다.

### 10.3 프로젝트 구조 검사

현재 Analyzer는 입력 프로젝트에 대해 다음을 검사한다.

**오브젝트**

- `objects`는 배열이어야 한다.
- 각 오브젝트는 문자열 `id`와 문자열 `name`을 가져야 한다.
- ID는 빈 문자열이거나 중복이면 안 된다.
- 스크립트에서 선택하는 오브젝트 이름은 정확히 하나만 일치해야 한다.

**변수 및 리스트**

- `variables`가 존재하면 배열이어야 한다.
- 각 항목에 문자열 `id`, 문자열 `name`이 필요하다.
- ID는 비어 있거나 중복이면 안 된다.
- `variableType`은 `variable`, `list`, `timer`, `answer`, `slide` 중 하나여야 한다.
- `object`는 누락/`null` 또는 프로젝트에 존재하는 오브젝트 ID 문자열이어야 한다.
- `value`는 존재할 경우 문자열이어야 한다.
- `visible`, `isCloud`는 존재할 경우 불리언이어야 한다.
- `x`, `y`, `width`, `height`, `minValue`, `maxValue`는 존재하고 `null`이 아닐 경우 숫자여야 한다.
- 리스트는 `array` 배열이 있어야 하며 각 항목은 문자열 `data`를 가져야 한다.
- 같은 범위(owner)가 같은 이름을 가진 변수/리스트를 중복 정의하면 안 된다.

**함수**

- `functions`가 존재하면 배열이어야 한다.
- 각 함수는 고유한 비어 있지 않은 문자열 `id`를 가져야 한다.
- `type`은 `normal` 또는 `value`여야 한다.
- `useLocalVariables`는 존재할 경우 불리언 또는 `null`이어야 한다.
- `localVariables`는 존재할 경우 배열 또는 `null`이어야 한다.
- `content`는 JSON 문자열이어야 한다.
- `content` 내부에 `function_create` 또는 `function_create_value` 정의 블록이 있어야 하며 `type`과 일치해야 한다.
- 함수 이름은 비어 있지 않고 프로젝트 안에서 중복되지 않아야 한다.
- 함수의 매개변수 리본 구조를 해석할 수 있어야 한다.

이 검사는 전체 Entry 프로젝트 JSON 스키마를 검증하는 완전한 validator가 아니다. 명시된 필드와 컴파일에 필요한 구조를 중심으로 검사한다.

---

## 11. 에셋과 `.ent` 패키징

### 11.1 에셋 루트

- `entscript build`: `<project-dir>/assets/`
- 직접 컴파일: 현재 작업 디렉터리의 `assets/`

에셋 폴더가 없고 프로젝트 JSON에 로컬 해시 경로 참조도 없다면 패키징을 진행할 수 있다. 참조가 있는데 실제 파일이 없으면 실패한다.

### 11.2 인식하는 Entry 에셋 경로

Entry 관리 에셋은 다음 상대 경로 형태로 인식한다.

```text
<2글자>/<2글자>/{image|thumb|sound}/<file-id>.<extension>
```

예시:

```text
ab/cd/image/abcd1234.png
ab/cd/thumb/abcd1234.png
ef/gh/sound/efgh5678.mp3
```

검사 규칙:

- 앞의 두 디렉터리는 각각 ASCII 영문자 또는 숫자 2글자여야 한다.
- 종류 폴더는 `image`, `thumb`, `sound` 중 하나여야 한다.
- 파일명은 `file-id.extension` 형식이어야 한다.
- 파일 ID는 ASCII 영문자/숫자만으로 구성되고 최소 4글자여야 한다.
- 파일 ID의 앞 네 글자는 두 디렉터리 이름과 대소문자 무시 기준으로 일치해야 한다.
- 확장자는 비어 있지 않은 ASCII 영문자/숫자 문자열이어야 한다. 코드가 종류별 확장자를 엄격히 제한하지는 않는다.

### 11.3 프로젝트 JSON 참조 검색

프로젝트 JSON 전체를 재귀적으로 탐색해 다음 이름의 문자열 필드를 찾는다.

- `fileurl`
- `thumbUrl`

값에서 다음 처리를 거쳐 Entry 경로를 추출한다.

- `?query`와 `#fragment` 이후 제거
- 역슬래시를 슬래시로 정규화
- 선행 슬래시와 URL 경로 구분자를 무시하고 네 구성요소 패턴 탐색
- 규칙에 맞는 첫 경로를 패키지 상대 경로로 사용

이 규칙에 맞지 않는 경로는 로컬 에셋으로 취급하지 않으며, 해당 문자열을 그대로 둔다. 파일 URL 전체를 다운로드하지 않는다.

### 11.4 에셋 복사 및 보안 검사

- JSON이 실제 참조하는 에셋만 복사한다.
- 참조되지 않는 파일은 복사하지 않는다.
- `assets/thumbnail.png` 같은 임의 파일은 참조 경로 규칙과 일치하지 않으면 복사하지 않는다.
- 참조 파일이 없으면 빌드가 실패한다.
- 에셋 루트는 심볼릭 링크가 아닌 실제 디렉터리여야 한다.
- 참조 파일 경로의 각 구성요소에 심볼릭 링크가 있으면 실패한다.
- 최종 참조 대상은 일반 파일이어야 한다.
- 참조 파일을 `temp/` 아래 같은 상대 경로에 복사한다.

### 11.5 `metadata.thumbnail`과 에셋의 차이

`metadata.thumbnail`은 프로젝트 최상위 JSON에 기록하는 문자열 메타데이터다. 에셋 수집기는 `thumbnail` 키를 에셋 경로로 처리하지 않는다. 로컬 파일을 복사하거나 Entry 서버로 업로드하지 않는다.

### 11.6 패키지 구조

`.ent`는 gzip 압축 TAR다. 아카이브 내부 루트는 `temp/`다.

```text
output.ent
└── temp/
    ├── project.json
    └── ab/
        └── cd/
            ├── image/abcd1234.png
            ├── thumb/abcd1234.png
            └── sound/abcd1234.mp3
```

CLI는 시스템 `tar` 명령을 `PATH`에서 실행해 `tar -czf`로 패키징한다. 임시 스테이징 디렉터리를 사용하고 작업 후 제거한다. 시스템 `tar` 실행을 사용할 수 없는 환경에서는 CLI 패키징이 실패한다.

---

## 12. Rust 및 WebAssembly API

### 12.1 Cargo 구성

- library crate types: `rlib`, `cdylib`
- binary: `entscript` (`src/main.rs`)
- 기본 feature: 없음
- 선택 feature: `wasm`
- `wasm` feature는 optional `wasm-bindgen` 의존성을 활성화한다.

### 12.2 공개 Rust API

```rust
pub fn compile_to_json(source: &str) -> Result<String, CompileError>;

pub fn compile_with_project_json(
    source: &str,
    project_json: &str,
) -> Result<String, CompileError>;

pub fn compile_with_project_value(
    source: &str,
    project: serde_json::Value,
) -> Result<String, CompileError>;
```

| 함수 | 설명 |
|---|---|
| `compile_to_json` | 기본 Entry 프로젝트 템플릿으로 컴파일 |
| `compile_with_project_json` | JSON 문자열로 받은 기존 프로젝트로 컴파일 |
| `compile_with_project_value` | 이미 파싱된 `serde_json::Value`로 컴파일 |

반환값은 `.ent`가 아니라 Entry 프로젝트 JSON 문자열이다.

### 12.3 기본 프로젝트 템플릿

기본 프로젝트에는 다음 데이터가 들어간다.

- 기본 메타데이터 및 화면 설정
- `엔트리봇` 오브젝트(ID `7y0y`)
- 기본 장면(ID `7dwq`)
- 기본 변수 `초시계`(timer), `대답`(answer)
- 빈 함수 배열
- 빈 메시지 및 테이블·확장 관련 배열/설정

기본 템플릿에 포함된 `엔트리봇` 이외 오브젝트는 자동 생성하지 않는다.

### 12.4 WebAssembly 표면

`src/lib.rs`에는 다음 조건에서 활성화되는 WASM 모듈이 있다.

```text
target_arch = "wasm32" AND feature = "wasm"
```

Export 함수:

- `wasm_api::compile(source)`
- `wasm_api::compile_with_project(source, project_json)`

빌드 대상 명령 예시는 다음과 같다.

```bash
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown --features wasm --lib
```

**중요한 제한:** 소스에는 WASM API와 Cargo feature가 정의되어 있지만, 이 명세 작성 시점에 WASM 타깃 빌드 성공을 검증하지 않았다. ID 생성에서 `rand::thread_rng()`를 사용하며 WASM 타깃의 난수 백엔드 호환성도 별도 빌드 확인이 필요하다. WASM API는 JSON 컴파일만 제공하고 CLI의 파일 시스템 접근·`tar` 실행·`.ent` 패키징 기능을 제공하지 않는다.

---

## 13. 컴파일과 생성 데이터의 동작

### 13.1 컴파일 단계

1. Lexer: 문자열을 토큰화한다.
2. Parser: 토큰을 AST로 변환한다.
3. Analyzer: 선언, 변수/리스트, 함수, 프로젝트 구조, 블록 이름·인자·문장 영역을 검사한다.
4. EntryBuilder: 변수·함수·오브젝트 스크립트를 Entry IR로 만든다.
5. Serializer: IR을 Entry 블록 JSON으로 직렬화한다.
6. CLI: 필요하면 JSON과 참조 에셋을 gzip TAR `.ent`로 묶는다.

### 13.2 블록 데이터

블록 ID는 4글자 소문자 영숫자(base 36) 후보를 난수로 생성하고, 입력 프로젝트에서 수집한 기존 ID와 충돌하지 않는 값을 선택한다. 수집 대상에는 일반 JSON의 `id` 필드와 `script`/`content` 문자열 내부에 직렬화된 JSON의 ID가 포함된다.

생성 블록의 기본 데이터 필드는 다음과 같다.

- `id`
- `x`, `y`
- `type`
- `params`
- `statements`
- `movable`, `deletable`, `emphasized`, `readOnly`, `copyable`, `assemble`, `extensions`

문자열과 숫자 입력은 Entry `text`/`number` 하위 블록으로 직렬화된다. 불리언은 `True`/`False` 블록으로 직렬화된다. `field()`와 `raw()`는 이 변환을 우회하는 입력 인코딩 도구다.

### 13.3 스크립트 저장 형식

오브젝트의 `script`와 함수의 `content`는 Entry 스레드 배열을 JSON 문자열로 저장한다. 각 스레드는 블록 배열이며, 각 블록은 `params` 및 `statements` 배열을 가진다.

---

## 14. 의미 분석과 타입 검사 범위

Analyzer가 수행하는 주요 검사는 다음과 같다.

- 소스에서 같은 이름의 전역 변수/리스트 선언 중복 검사
- 기존 프로젝트 변수 이름과 새 선언의 충돌 검사
- 초기 변수/리스트 항목이 리터럴인지 검사
- 소스 함수 이름·매개변수 중복 검사
- 내장 블록 이름과 사용자 함수 이름 충돌 검사
- 오브젝트 이름 존재 및 유일성 검사
- 변수와 리스트 이름 해석 및 종류 구분
- 리스트 연산 인자 형태 검사
- 사용자 함수 인자 개수 검사
- 기존 프로젝트 함수의 매개변수 개수 및 분석 가능한 타입 검사
- 등록된 블록/이벤트의 인자 개수 검사
- 문장 영역이 있는 블록에만 `{}` 또는 `else {}`를 허용
- 일부 조건식과 불리언 함수 인자에 대한 제한적인 불리언 검사
- 프로젝트 오브젝트, 변수, 리스트, 함수의 필수 구조와 일부 자료형 검사

### 14.1 타입 검사의 한계

현재 언어에는 완전한 정적 타입 시스템이 없다.

- 숫자, 문자열, 불리언 등 모든 표현식의 타입을 완전하게 추론하지 않는다.
- 계산 연산자에 전달되는 값의 의미상 자료형을 포괄적으로 검증하지 않는다.
- Entry 리포터의 반환 타입 정보는 일부 알려진 블록만 불리언으로 분류한다.
- 조건식에서 타입이 명확히 값으로 판단되면 거부하지만, 타입을 판단할 수 없는 식은 통과할 수 있다.
- 함수 `bool` 매개변수 검사는 현재 내장 추론 규칙에 한정된다.
- Entry 자체에서 실행될 때의 자료형 오류·드롭다운 값 오류·런타임 오류를 보장해서 잡지 않는다.

---

## 15. 명시적 미지원 목록

아래 기능은 현재 구현에 없거나, 일부 기능만 존재하므로 지원으로 간주하면 안 된다.

### 15.1 언어 문법 미지원

- 세미콜론 문장 종료자
- 블록 주석 `/* ... */`
- 작은따옴표 문자열
- 여러 줄 문자열
- 문자열 보간/템플릿 문자열
- 지수·16진수 숫자 표기
- unary plus
- 비트 연산자 및 시프트 연산자
- 대입 표현식, 복합 대입 연산자 (`+=` 등)
- 일반 배열 리터럴/배열 타입 (리스트 선언 전용 `[...]`는 지원)
- 인덱스 접근 문법 (`items[0]`)
- 멤버 접근/속성 접근 (`object.property`)
- 클래스, 구조체, enum, 모듈 선언
- `import`, `include`, `use`
- `let`, `const` 등 `var` 외 변수 선언 키워드
- `return` 문
- 중첩 함수 선언
- 람다 및 익명 함수
- 함수 오버로딩, 기본/가변 인자, 이름 있는 인자
- 함수 지역 변수 선언
- 참조·포인터·출력 매개변수
- 정수/실수/문자 등 별도 선언 타입 시스템
- 일반적인 사용자 정의 타입 및 타입 별칭
- 예외 처리/오류 복구 구문
- 비동기/await 문법
- 전처리기, 매크로, 제네릭

### 15.2 프로젝트 및 빌드 미지원

- 여러 `.ents` 파일의 자동 발견·병합
- `modules/` 자동 로딩 또는 import
- 모듈 간 의존성/빌드 순서 관리
- 소스 파일별 증분 컴파일 및 캐시
- `settings.json` 외부 설정 버전 1 이외의 버전
- `metadata.thumbnail` 파일 복사, 이미지 임베드, Entry 서버 업로드
- Entry 계정 인증 및 프로젝트 직접 업로드
- 원격 에셋 URL 다운로드
- 해시 경로 규칙 밖의 임의 에셋 자동 복사
- 완전한 Entry 프로젝트 JSON 스키마 검증
- 테이블 ID·열·행 존재 여부 사전 검증

### 15.3 Entry 기능 미지원 또는 제한

- 인공지능 블록 및 AI 모델 학습/설정
- 확장 모듈의 설치·초기화·런타임 연결
- 하드웨어·보드·로봇의 연결과 장치 런타임
- Entry의 모든 네이티브 블록에 대한 전용 친숙한 별칭
- 모든 네이티브 블록 파라미터 구조의 자동 검색
- `raw_block()`의 인자 개수/슬롯 타입 자동 검증
- 모든 Entry 버전에서의 블록 ID·파라미터 호환성 보장
- Entry 편집기/게시 화면의 메타데이터 필드 표시 보장
- 런타임 동작의 시뮬레이션 또는 실행 검증

미등록 일반 블록은 `raw_block()`으로 JSON에 기록할 수 있는 경우가 있지만, 해당 블록이 Entry 런타임에서 실제로 작동한다는 의미는 아니다.

### 15.4 WASM 제한

- WASM CLI 파일 입출력 및 `.ent` 패키징
- WASM에서 `tar` 프로세스 실행
- WASM API의 빌드 성공은 현재 검증되지 않음
- WASM 난수 백엔드의 타깃 호환성은 별도 검증 필요

---

## 16. 오류 종류

컴파일러는 다음 범주의 오류를 구분한다.

| 범주 | 발생 단계 | 예 |
|---|---|---|
| Lexer | 토큰화 | 알 수 없는 문자, 잘못된 문자열 이스케이프, 닫히지 않은 문자열 |
| Parser | 구문 분석 | 최상위 선언 위치 오류, 괄호/중괄호 누락, 잘못된 인자 구문 |
| Semantic | 의미 분석/빌드 | 미등록 블록, 인자 개수 오류, 변수·리스트·함수 불일치 |
| Project | 기존 프로젝트 검사 | `objects` 누락, 중복 ID, 잘못된 리스트 배열, 잘못된 함수 content |
| JSON | JSON 파싱 | 기존 프로젝트 JSON 파싱 오류 |
| CLI/I/O | 파일·패키징 | 소스/설정/프로젝트 파일 읽기 오류, 에셋 누락, TAR 실패 |

Lexer/Parser 오류에는 줄·열 정보가 포함된다. Semantic/Project/CLI 오류는 현재 구현에서 항상 소스 위치를 추적하지 않는다.

---

## 17. 현재 저장소의 예제와 테스트

예제 파일:

- `examples/hello.ents`: 이벤트, 말하기, 반복, 대기, 조건
- `examples/features.ents`: 전역 변수, 리스트, 사용자 함수, 값 함수, 변수/리스트 연산
- `examples/wiki_blocks.ents`: 움직임, 붓, 글상자, 데이터 분석 리포터 등의 일부 별칭
- `examples/project/`: `settings.json`을 사용하는 빌드 예제

테스트 파일:

- `tests/pipeline.rs`: 기본 컴파일, ID 유일성, 프로젝트 데이터 보존, 알 수 없는 오브젝트/블록 오류
- `tests/language_features.rs`: 변수, 리스트, 함수, 매개변수, 전방 호출, 프로젝트 지역 변수 범위 및 프로젝트 함수 검사
- `tests/native_blocks.rs`: 제어 블록, 조건 블록, `raw_block`, 변수, 네이티브 타입 ID, AI/확장 블록 거부
- `tests/wiki_blocks.rs`: 움직임, 붓, 글상자, 소리, 계산, 날짜, 데이터 분석 테이블 블록

테스트가 존재한다는 사실이 Entry 웹 런타임에서 모든 블록의 동작까지 검증했다는 뜻은 아니다. 현재 테스트는 주로 컴파일 결과 JSON 구조와 오류 처리를 확인한다.

---

## 18. 참고할 구현 파일

| 파일 | 역할 |
|---|---|
| `src/lexer.rs` | 문자 처리, 토큰, 문자열·숫자·식별자·주석 |
| `src/parser.rs` | 최상위 선언, 함수, 오브젝트, 문장, 표현식 파싱 |
| `src/ast.rs` | AST 자료구조 |
| `src/analyzer.rs` | 의미 분석, 프로젝트 구조 검증, 심볼 해석 |
| `src/entry/registry.rs` | 이벤트·문장·리포터 블록 이름과 파라미터 사양 |
| `src/entry/builder.rs` | AST를 Entry IR로 변환 |
| `src/entry/ir.rs` | Entry 블록/스레드/스크립트 IR |
| `src/entry/input.rs` | Entry 입력 인코딩 종류 |
| `src/entry/serializer.rs` | IR을 Entry JSON으로 변환하고 ID 생성 |
| `src/entry/project/` | 기본 프로젝트 템플릿 |
| `src/lib.rs` | 공개 Rust API 및 WASM export |
| `src/main.rs` | CLI, 설정 파싱, 에셋 복사, `.ent` 패키징 |

이 명세와 실제 코드가 달라지면 실제 구현이 우선한다. 기능을 변경할 때는 이 문서의 지원/미지원 상태와 예제를 함께 갱신해야 한다.
