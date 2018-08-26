bin :=
NAME := $(bin)
fea := $(shell grep "\[\[bin\]\]" -A3 Cargo.toml | grep $(NAME) -A2 | grep required | awk -F'[][]' '{print $$2}')
FEATURES := $(if $(fea),"--features=$(fea)",)
release :=
MODE := $(if $(release),release,debug)
RELEASE_FLAG := $(if $(release),--release,)
TARGET := ./target/thumbv7em-none-eabihf/$(MODE)
BIN := $(TARGET)/$(NAME)

UNAME := $(shell uname)
ifeq ($(UNAME), Linux)
TTY := /dev/ttyUSB0
endif
ifeq ($(UNAME), Darwin)
TTY := /dev/tty.wchusbserial1420
endif

$(BIN): build

$(BIN).bin: $(BIN)
	arm-none-eabi-objcopy -S -O binary $(BIN) $(BIN).bin

build:
	cargo -v build $(RELEASE_FLAG) --bin $(NAME) $(FEATURES)

flash: $(BIN).bin
	python2 ./loader/stm32loader.py -p $(TTY) -f F3 -e -w -v $(BIN).bin

load: flash

clean:
	cargo -v clean

.PHONY: build
