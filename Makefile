bin :=
NAME := $(if $(bin),$(bin),dflt)
ifndef NAME
$(error Set bin to build, e.g. 'bin=mini')
endif
fea := $(shell grep "\[\[bin\]\]" -A3 Cargo.toml | grep $(NAME) -A2 | grep required | awk -F'[][]' '{print $$2}')
FEATURES := $(if $(fea),"--features=$(fea)",)
release :=
MODE := $(if $(release),release,debug)
RELEASE_FLAG := $(if $(release),--release,)
target :=
TARGET := $(if $(target),"$(target)",thumbv7em-none-eabihf)
TARGET_PATH := ./target/$(TARGET)/$(MODE)
BIN := $(TARGET_PATH)/$(NAME)
mem :=
MEM := $(if $(mem),$(mem),128k)

ifeq (,$(wildcard memory.$(MEM)))
$(error File memory.$(MEM) do not exist, create if you want to use different memory settings)
endif

UNAME := $(shell uname)
ifeq ($(UNAME), Linux)
TTY := /dev/ttyUSB0
endif
ifeq ($(UNAME), Darwin)
TTY := /dev/tty.usbserial-1410
endif

$(BIN): build

$(BIN).bin: $(BIN)
	arm-none-eabi-objcopy -S -O binary $(BIN) $(BIN).bin

# not using memory.x: to allow overriding
memory:
	cp memory.$(MEM) memory.x

build: memory
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

bloat:
	cargo -v bloat --bin $(NAME) $(FEATURES) $(RELEASE_FLAG) --crates

clean:
	cargo -v clean
	rm memory.x

goad: build
	sh -c "openocd & arm-none-eabi-gdb -q $(BIN) & wait"

nodevice: memory
	cargo -v build --bins --features=no_device

all: memory
	cargo -v build --bins --features=all

# Problem:
#   openocd does not connect (init failed, unknown status code 0x09).
#   Can be connected when holding down NRST pin (via button or with jumper wire).
#   Can't flash while NRST is down (target not halted), but openocd connection breaks immediately as soon as jumper wire is removed.
# The fix:
#   'reset_config connect_assert_srst' forces reset state when openocd starts.
#   'reset halt' issues halt cmd and (AIUI) deasserts SRST so cpu leaves reset state and halts.
#   'stm32f3x.cpu curstate' to check that we actually halted.
#   After this can follow any other command to fix whatever mess was there.
fix-nucleo:
	openocd -f openocd.cfg -c 'reset_config connect_assert_srst srst_only' -c init -c 'reset halt' -c 'stm32f3x.cpu curstate' -c 'stm32f1x mass_erase 0'

.PHONY: build
