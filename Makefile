bin :=
NAME := $(bin)
fea := $(shell grep "\[\[bin\]\]" -A3 Cargo.toml | grep $(NAME) -A2 | grep required | awk -F'[][]' '{print $$2}')
FEATURES := $(if $(fea),"--features=$(fea)",)
release :=
MODE := $(if $(release),release,debug)
RELEASE_FLAG := $(if $(release),--release,)
target :=
TARGET := $(if $(target),"$(target)",thumbv7em-none-eabihf)
TARGET_PATH := ./target/$(TARGET)/$(MODE)
BIN := $(TARGET_PATH)/$(NAME)

UNAME := $(shell uname)
ifeq ($(UNAME), Linux)
TTY := /dev/ttyUSB0
endif
ifeq ($(UNAME), Darwin)
TTY := /dev/tty.wchusbserial1410
endif

$(BIN): build

$(BIN).bin: $(BIN)
	arm-none-eabi-objcopy -S -O binary $(BIN) $(BIN).bin

build:
	cargo -v build $(RELEASE_FLAG) --target $(TARGET) --bin $(NAME) $(FEATURES)

flash: $(BIN).bin
	python2 ./loader/stm32loader.py -p $(TTY) -f F3 -e -w $(BIN).bin

load: flash

boad: build
	bobbin -v load $(RELEASE_FLAG) --target $(TARGET) --bin $(NAME) $(FEATURES)

brun: build
	bobbin -v run --bin $(NAME) $(FEATURES)

crun: build
	cargo -v run --bin $(NAME) $(FEATURES)

clean:
	cargo -v clean

gdbload: build
	sh -c "openocd & arm-none-eabi-gdb -q $(BIN) & wait"

.PHONY: build
