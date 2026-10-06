#ifndef CAMPFIRE_ONIGMO_CONFIG_H
#define CAMPFIRE_ONIGMO_CONFIG_H
#define PACKAGE "Smartfire Ruby 3.4.10 Onigmo"
#define HAVE_STDINT_H 1
#define HAVE_INTTYPES_H 1
#define HAVE_SYS_TYPES_H 1
#define HAVE_STDLIB_H 1
#define HAVE_LIMITS_H 1
#define HAVE_ALLOCA_H 1
#define SIZEOF_LONG __SIZEOF_LONG__
#define SIZEOF_INT __SIZEOF_INT__
#define SIZEOF_LONG_LONG __SIZEOF_LONG_LONG__
#define SIZEOF_VOIDP __SIZEOF_POINTER__
#define RUBY_SYMBOL_EXPORT_BEGIN
#define RUBY_SYMBOL_EXPORT_END
#include <stdint.h>
#define UNREACHABLE_RETURN(value) return (value)
#define RB_GNUC_EXTENSION __extension__
#define RB_GNUC_EXTENSION_BLOCK(x) __extension__ ({x;})
#define MEMCPY(a,b,t,n) memcpy((a),(b),sizeof(t)*(n))
#define nlz_intptr(x) ((x) ? __builtin_clzl(x) : (8 * sizeof(unsigned long)))
#define NO_SANITIZE(attribute, declaration) declaration
#endif
